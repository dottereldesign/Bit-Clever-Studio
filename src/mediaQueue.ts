export type MediaProgress = { total: number; completed: number; failed: number; pending: number };
type Job = {
  key: string;
  thumbnail: boolean;
  priority:number;
  consumers: Map<AbortSignal | undefined, number>;
  run: () => Promise<string>;
  resolve: (value: string) => void;
  reject: (error: unknown) => void;
  promise: Promise<string>;
};

// Share native work across StrictMode remounts and rapid gallery navigation.
// Aborting one card must never cancel another card's request for the same file.
export class MediaQueue {
  private active = 0;
  private backgroundActive = 0;
  private dirty = false;
  private scheduled = false;
  private jobs = new Map<string, Job>();
  private queue: Job[] = [];
  private listeners = new Set<() => void>();
  private progress: MediaProgress = { total: 0, completed: 0, failed: 0, pending: 0 };
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };
  getProgress = () => this.progress;
  private update(patch: Partial<MediaProgress>) {
    this.progress = { ...this.progress, ...patch };
    this.listeners.forEach(listener => listener());
  }
  request(key: string, thumbnail: boolean, run: () => Promise<string>, signal?: AbortSignal, priority=1): Promise<string> {
    if (signal?.aborted) return Promise.reject(new Error("Cancelled"));
    let job = this.jobs.get(key);
    if (!job) {
      let resolve!: Job['resolve'], reject!: Job['reject'];
      const promise = new Promise<string>((yes, no) => { resolve = yes; reject = no; });
      job = { key, thumbnail, priority:thumbnail?priority:-1, run, consumers: new Map(), resolve, reject, promise };
      this.jobs.set(key, job);
      if (thumbnail) {
        const p = this.progress;
        this.update(p.pending ? { total: p.total + 1, pending: p.pending + 1 } : { total: 1, pending: 1, completed: 0, failed: 0 });
        this.queue.push(job);
      } else this.queue.unshift(job);
    }
    job.consumers.set(signal, Math.min(job.consumers.get(signal) ?? Infinity, thumbnail ? priority : -1));
    this.dirty = true;
    const result = !signal ? job.promise : new Promise<string>((resolve, reject) => {
      const abort = () => { reject(new Error("Cancelled")); this.dirty=true; this.schedule(); };
      signal.addEventListener('abort', abort, { once: true });
      job!.promise.then(value => {
        if (!signal.aborted) resolve(value);
      }, reject).finally(() => signal.removeEventListener('abort', abort));
    });
    if(thumbnail&&priority>0)this.schedule();else this.pump();
    return result;
  }
  prioritize(key:string,priority:number){
    const job=this.jobs.get(key);
    if(job){
      // A mounted card can change priority; a background subscriber stays background.
      for(const [signal,value] of job.consumers)if(value<2)job.consumers.set(signal,priority);
      this.dirty=true;this.schedule();
    }
  }
  private schedule(){
    if(this.scheduled)return;this.scheduled=true;
    queueMicrotask(()=>{this.scheduled=false;this.pump();});
  }
  private pump() {
    if(this.dirty){
      this.dirty=false;
      let cancelled=0;
      this.queue=this.queue.filter(job=>{
        const priorities=[...job.consumers].filter(([signal])=>!signal?.aborted).map(([,value])=>value);
        if(!priorities.length){this.jobs.delete(job.key);if(job.thumbnail)cancelled++;job.reject(new Error("Cancelled"));return false;}
        job.priority=Math.min(...priorities);return true;
      });
      if(cancelled)this.update({total:this.progress.total-cancelled,pending:this.progress.pending-cancelled});
      this.queue.sort((a,b)=>a.priority-b.priority);
    }
    while (this.active < 3 && this.queue.length) {
      const index=this.queue.findIndex(job=>job.priority<2||this.backgroundActive<1);
      if(index<0)break;
      const [job]=this.queue.splice(index,1);
      const background=job.priority>=2;
      if(background)this.backgroundActive++;
      this.active++;
      void Promise.resolve().then(job.run).then(job.resolve, error => {
        if (job.thumbnail) this.update({ failed: this.progress.failed + 1 });
        job.reject(error);
      }).finally(() => {
        this.jobs.delete(job.key);
        this.active--;
        if(background)this.backgroundActive--;
        if (job.thumbnail) this.update({ completed: this.progress.completed + 1, pending: this.progress.pending - 1 });
        this.pump();
      });
    }
  }
}
