import { ThinWedgeOptions } from "./thinwedgeOptions";
import { ThinWedgeExec } from "./exec";
import { Thread } from "./thread";
import { ThreadOptions } from "./threadOptions";

/**
 * ThinWedge is the main class for interacting with the ThinWedge agent.
 *
 * Use the `startThread()` method to start a new thread or `resumeThread()` to resume a previously started thread.
 */
export class ThinWedge {
  private exec: ThinWedgeExec;
  private options: ThinWedgeOptions;

  constructor(options: ThinWedgeOptions = {}) {
    const { thinwedgePathOverride, env, config } = options;
    this.exec = new ThinWedgeExec(thinwedgePathOverride, env, config);
    this.options = options;
  }

  /**
   * Starts a new conversation with an agent.
   * @returns A new thread instance.
   */
  startThread(options: ThreadOptions = {}): Thread {
    return new Thread(this.exec, this.options, options);
  }

  /**
   * Resumes a conversation with an agent based on the thread id.
   * Threads are persisted in ~/.thinwedge/sessions.
   *
   * @param id The id of the thread to resume.
   * @returns A new thread instance.
   */
  resumeThread(id: string, options: ThreadOptions = {}): Thread {
    return new Thread(this.exec, this.options, options, id);
  }
}
