#!/usr/bin/env -S NODE_NO_WARNINGS=1 pnpm ts-node-esm --files

import { ThinWedge } from "@thinwedge/thinwedge-sdk";
import { thinwedgePathOverride } from "./helpers.ts";
import z from "zod";
import zodToJsonSchema from "zod-to-json-schema";

const thinwedge = new ThinWedge({ thinwedgePathOverride: thinwedgePathOverride() });
const thread = thinwedge.startThread();

const schema = z.object({
  summary: z.string(),
  status: z.enum(["ok", "action_required"]),
});

const turn = await thread.run("Summarize repository status", {
  outputSchema: zodToJsonSchema(schema, { target: "thinwedgeAi" }),
});
console.log(turn.finalResponse);
