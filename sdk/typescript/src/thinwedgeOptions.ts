export type ThinWedgeConfigValue = string | number | boolean | ThinWedgeConfigValue[] | ThinWedgeConfigObject;

export type ThinWedgeConfigObject = { [key: string]: ThinWedgeConfigValue };

export type ThinWedgeOptions = {
  thinwedgePathOverride?: string;
  baseUrl?: string;
  apiKey?: string;
  /**
   * Additional `--config key=value` overrides to pass to the ThinWedge CLI.
   *
   * Provide a JSON object and the SDK will flatten it into dotted paths and
   * serialize values as TOML literals so they are compatible with the CLI's
   * `--config` parsing.
   */
  config?: ThinWedgeConfigObject;
  /**
   * Environment variables passed to the ThinWedge CLI process. When provided, the SDK
   * will not inherit variables from `process.env`.
   */
  env?: Record<string, string>;
};
