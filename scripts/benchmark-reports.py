"""Compare the old and indexed/joined report query over the same long history."""
import datetime
import json
import sqlite3
import statistics
import time
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "artifacts" / f"report-benchmark-{time.time_ns()}.db"
path.parent.mkdir(parents=True, exist_ok=True)
db = sqlite3.connect(path)
db.executescript("""
PRAGMA cache_size=-1024; PRAGMA mmap_size=0;
CREATE TABLE intervals(id INTEGER PRIMARY KEY,session_id INTEGER,start_ms INTEGER,end_ms INTEGER,active_ms INTEGER);
CREATE TABLE sessions(id INTEGER PRIMARY KEY,completed INTEGER);
CREATE INDEX intervals_range ON intervals(start_ms,end_ms);
""")
midnight = int(datetime.datetime.now().astimezone().replace(hour=0, minute=0, second=0, microsecond=0).timestamp()*1000)
with db:
    db.executemany("INSERT INTO intervals VALUES(?,?,?,?,1000)",
        ((n, n, midnight-2_000_000_000+n*1000, midnight-2_000_000_000+n*1000+1000) for n in range(1,1_000_001)))
    db.executemany("INSERT INTO intervals VALUES(?,?,?,?,1000)",
        ((1_000_000+n,n,midnight+n*1000,midnight+n*1000+1000) for n in range(1,10_001)))
    db.executemany("INSERT INTO sessions VALUES(?,1)", ((n,) for n in range(1,10_001)))
old = "SELECT start_ms,end_ms,active_ms,session_id FROM intervals WHERE start_ms<?2 AND end_ms>?1"
new = "SELECT i.start_ms,i.end_ms,i.active_ms,COALESCE(s.completed,0) FROM intervals i INDEXED BY intervals_end LEFT JOIN sessions s ON s.id=i.session_id WHERE i.end_ms>?1 AND i.start_ms<?2"


def bench(sql, legacy):
    results = []
    for _ in range(9):
        started = time.perf_counter()
        count = 0
        for a,b,active,session in db.execute(sql, (midnight,midnight+86400000)):
            if legacy:
                db.execute("SELECT completed FROM sessions WHERE id=?", (session,)).fetchone()
            count += active
        assert count == 10_000_000
        results.append((time.perf_counter()-started)*1000)
    return dict(medianMs=statistics.median(results[1:]), samplesMs=results,
                queryPlan=list(db.execute("EXPLAIN QUERY PLAN "+sql,(midnight,midnight+86400000))))

report = dict(oldIntervals=1_000_000, queriedIntervals=10_000, cacheKiB=1024, before=bench(old,True))
db.execute("CREATE INDEX intervals_end ON intervals(end_ms)")
report["after"] = bench(new,False)
report["speedup"] = report["before"]["medianMs"]/report["after"]["medianMs"]
(root / "artifacts/report-benchmark.json").write_text(json.dumps(report,indent=2))
print(json.dumps(report,indent=2))
