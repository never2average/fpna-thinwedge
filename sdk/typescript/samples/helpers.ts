import path from "node:path";

export function thinwedgePathOverride() {
  return (
    process.env.THINWEDGE_EXECUTABLE ??
    path.join(process.cwd(), "..", "..", "thinwedge-rs", "target", "debug", "thinwedge")
  );
}
