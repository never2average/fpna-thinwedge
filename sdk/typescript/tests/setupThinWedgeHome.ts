import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { afterEach, beforeEach } from "@jest/globals";

const originalThinWedgeHome = process.env.THINWEDGE_HOME;
let currentThinWedgeHome: string | undefined;

beforeEach(async () => {
  currentThinWedgeHome = await fs.mkdtemp(path.join(os.tmpdir(), "thinwedge-sdk-test-"));
  process.env.THINWEDGE_HOME = currentThinWedgeHome;
});

afterEach(async () => {
  const thinwedgeHomeToDelete = currentThinWedgeHome;
  currentThinWedgeHome = undefined;

  if (originalThinWedgeHome === undefined) {
    delete process.env.THINWEDGE_HOME;
  } else {
    process.env.THINWEDGE_HOME = originalThinWedgeHome;
  }

  if (thinwedgeHomeToDelete) {
    await fs.rm(thinwedgeHomeToDelete, { recursive: true, force: true });
  }
});
