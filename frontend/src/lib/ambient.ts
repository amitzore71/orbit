import type { AmbientSound } from './protocol';

/**
 * Offline procedural ambient soundscape engine using Web Audio API.
 * Zero audio files, zero network calls, lazy AudioContext creation,
 * and automatic suspend() when inactive for 0% idle CPU.
 */
export class AmbientEngine {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private activeNodes: AudioNode[] = [];
  private activeOscs: (OscillatorNode | AudioBufferSourceNode)[] = [];
  private currentPreset: AmbientSound = 'off';
  private playing = false;
  private suspendTimer: ReturnType<typeof setTimeout> | null = null;

  private ensureContext(): AudioContext | null {
    if (this.ctx) return this.ctx;
    if (typeof window === 'undefined') return null;
    const Ctor =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Ctor) return null;
    try {
      const ctx = new Ctor();
      const master = ctx.createGain();
      master.gain.setValueAtTime(0, ctx.currentTime);
      master.connect(ctx.destination);
      this.ctx = ctx;
      this.master = master;
      return ctx;
    } catch {
      return null;
    }
  }

  private clearGraph() {
    for (const src of this.activeOscs) {
      try {
        src.stop();
      } catch {
        /* already stopped */
      }
      try {
        src.disconnect();
      } catch {
        /* ignore */
      }
    }
    for (const node of this.activeNodes) {
      try {
        node.disconnect();
      } catch {
        /* ignore */
      }
    }
    this.activeOscs = [];
    this.activeNodes = [];
  }

  private makeNoiseBuffer(ctx: AudioContext, kind: 'brown' | 'pink'): AudioBuffer {
    const length = ctx.sampleRate * 4;
    const buffer = ctx.createBuffer(2, length, ctx.sampleRate);
    for (let ch = 0; ch < 2; ch++) {
      const data = buffer.getChannelData(ch);
      if (kind === 'brown') {
        let last = 0;
        for (let i = 0; i < length; i++) {
          const white = Math.random() * 2 - 1;
          last = (last + 0.02 * white) / 1.02;
          data[i] = last * 3.2;
        }
      } else {
        let b0 = 0, b1 = 0, b2 = 0, b3 = 0, b4 = 0, b5 = 0, b6 = 0;
        for (let i = 0; i < length; i++) {
          const white = Math.random() * 2 - 1;
          b0 = 0.99886 * b0 + white * 0.0555179;
          b1 = 0.99332 * b1 + white * 0.0750759;
          b2 = 0.96900 * b2 + white * 0.1538520;
          b3 = 0.86650 * b3 + white * 0.3104856;
          b4 = 0.55000 * b4 + white * 0.5329522;
          b5 = -0.7616 * b5 - white * 0.0168980;
          data[i] = (b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362) * 0.09;
          b6 = white * 0.115926;
        }
      }
      // Crossfade the last 2048 samples into the start for a seamless loop.
      const fade = 2048;
      for (let i = 0; i < fade; i++) {
        const t = i / fade;
        data[i] = data[i] * t + data[length - fade + i] * (1 - t);
      }
    }
    return buffer;
  }

  private buildPreset(ctx: AudioContext, master: GainNode, preset: AmbientSound) {
    this.clearGraph();
    if (preset === 'off') return;

    if (preset === 'brown_noise') {
      const src = ctx.createBufferSource();
      src.buffer = this.makeNoiseBuffer(ctx, 'brown');
      src.loop = true;
      const filter = ctx.createBiquadFilter();
      filter.type = 'lowpass';
      filter.frequency.value = 380;
      filter.Q.value = 0.7;
      src.connect(filter);
      filter.connect(master);
      src.start();
      this.activeOscs.push(src);
      this.activeNodes.push(filter);
      return;
    }

    if (preset === 'rain') {
      const src = ctx.createBufferSource();
      src.buffer = this.makeNoiseBuffer(ctx, 'pink');
      src.loop = true;
      const lp = ctx.createBiquadFilter();
      lp.type = 'lowpass';
      lp.frequency.value = 1400;
      const hp = ctx.createBiquadFilter();
      hp.type = 'highpass';
      hp.frequency.value = 180;
      const lfo = ctx.createOscillator();
      lfo.frequency.value = 0.14;
      const lfoGain = ctx.createGain();
      lfoGain.gain.value = 260;
      lfo.connect(lfoGain);
      lfoGain.connect(lp.frequency);
      src.connect(hp);
      hp.connect(lp);
      lp.connect(master);
      src.start();
      lfo.start();
      this.activeOscs.push(src, lfo);
      this.activeNodes.push(hp, lp, lfoGain);
      return;
    }

    if (preset === 'cafe') {
      const src = ctx.createBufferSource();
      src.buffer = this.makeNoiseBuffer(ctx, 'pink');
      src.loop = true;
      const bp = ctx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.value = 460;
      bp.Q.value = 0.65;
      const warmth = ctx.createBiquadFilter();
      warmth.type = 'lowshelf';
      warmth.frequency.value = 220;
      warmth.gain.value = 5;
      src.connect(bp);
      bp.connect(warmth);
      warmth.connect(master);
      src.start();
      this.activeOscs.push(src);
      this.activeNodes.push(bp, warmth);
      return;
    }

    if (preset === 'drone') {
      const filter = ctx.createBiquadFilter();
      filter.type = 'lowpass';
      filter.frequency.value = 340;
      filter.Q.value = 1.2;
      const subGain = ctx.createGain();
      subGain.gain.value = 0.24;
      const lfo = ctx.createOscillator();
      lfo.frequency.value = 0.08;
      const lfoGain = ctx.createGain();
      lfoGain.gain.value = 90;
      lfo.connect(lfoGain);
      lfoGain.connect(filter.frequency);
      lfo.start();
      this.activeOscs.push(lfo);

      const freqs: [number, OscillatorType][] = [
        [110, 'sine'],
        [164.81, 'sine'],
        [220.4, 'triangle'],
      ];
      for (const [freq, type] of freqs) {
        const osc = ctx.createOscillator();
        osc.type = type;
        osc.frequency.value = freq;
        osc.connect(filter);
        osc.start();
        this.activeOscs.push(osc);
      }
      filter.connect(subGain);
      subGain.connect(master);
      this.activeNodes.push(filter, subGain, lfoGain);
    }
  }

  sync(preset: AmbientSound, volumePercent: number, shouldPlay: boolean) {
    if (this.suspendTimer) {
      clearTimeout(this.suspendTimer);
      this.suspendTimer = null;
    }
    const wantAudio = shouldPlay && preset !== 'off' && volumePercent > 0;
    if (!wantAudio) {
      this.playing = false;
      if (this.ctx && this.master) {
        const now = this.ctx.currentTime;
        this.master.gain.cancelScheduledValues(now);
        this.master.gain.setValueAtTime(this.master.gain.value, now);
        this.master.gain.linearRampToValueAtTime(0, now + 0.45);
        this.suspendTimer = setTimeout(() => {
          this.clearGraph();
          this.currentPreset = 'off';
          if (this.ctx && this.ctx.state === 'running') {
            void this.ctx.suspend().catch(() => {});
          }
        }, 480);
      }
      return;
    }

    const ctx = this.ensureContext();
    if (!ctx || !this.master) return;
    if (ctx.state === 'suspended') {
      void ctx.resume().catch(() => {});
    }
    if (this.currentPreset !== preset || this.activeOscs.length === 0) {
      this.currentPreset = preset;
      this.buildPreset(ctx, this.master, preset);
    }
    const targetGain = Math.max(0, Math.min(1, volumePercent / 100)) * 0.42;
    const now = ctx.currentTime;
    this.master.gain.cancelScheduledValues(now);
    this.master.gain.setValueAtTime(this.master.gain.value, now);
    this.master.gain.linearRampToValueAtTime(targetGain, now + (this.playing ? 0.15 : 0.6));
    this.playing = true;
  }

  dispose() {
    if (this.suspendTimer) {
      clearTimeout(this.suspendTimer);
      this.suspendTimer = null;
    }
    this.clearGraph();
    if (this.ctx) {
      void this.ctx.close().catch(() => {});
      this.ctx = null;
      this.master = null;
    }
    this.currentPreset = 'off';
    this.playing = false;
  }
}
