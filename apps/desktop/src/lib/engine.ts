/**
 * The typed client for the `ttucs serve` contract.
 *
 * This is the single place the UI talks to the engine. Every response is validated with the
 * zod schemas in `schema.ts`, the token travels only in `X-Ttucs-Token`, and every failure
 * becomes an {@link EngineError} with a `kind` the UI can branch on and a message it can
 * show verbatim.
 */
import type { ZodType } from "zod";

import {
  HealthSchema,
  ParseResponseSchema,
  PredictResponseSchema,
  ShutdownResponseSchema,
  TaxonomyEntryResponseSchema,
  TaxonomySearchResponseSchema,
  isSupportedSchemaVersion,
  type Health,
  type Prediction,
  type SlateFields,
  type TaxonomyEntry,
} from "./schema.ts";

/** Where the engine listens and the token to present. */
export interface Endpoint {
  /** Origin only, for example `http://127.0.0.1:8731`. */
  readonly baseUrl: string;
  /** The token from `serve.json`. */
  readonly token: string;
}

/** The shape of `fetch` this client needs, so tests can pass a stub. */
export type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

/** Why a request failed, in the terms the UI branches on. */
export type EngineErrorKind =
  | "transport"
  | "unauthorised"
  | "too_large"
  | "bad_request"
  | "not_found"
  | "schema"
  | "status";

/** A failed engine call. */
export class EngineError extends Error {
  /** The branch the UI switches on. */
  readonly kind: EngineErrorKind;
  /** The HTTP status, when a response was received. */
  readonly status: number | undefined;

  constructor(kind: EngineErrorKind, message: string, status?: number) {
    super(message);
    this.name = "EngineError";
    this.kind = kind;
    this.status = status;
  }
}

/** Options for one client. */
export interface ClientOptions {
  /** The endpoint to call. */
  readonly endpoint: Endpoint;
  /** The fetch implementation; defaults to the global one. */
  readonly fetch?: FetchLike;
}

/** Map an HTTP status to the error kind the UI shows. */
function kindForStatus(status: number): EngineErrorKind {
  switch (status) {
    case 400:
      return "bad_request";
    case 401:
      return "unauthorised";
    case 404:
      return "not_found";
    case 413:
      return "too_large";
    default:
      return "status";
  }
}

/** A client bound to one endpoint. */
export class EngineClient {
  readonly #endpoint: Endpoint;
  readonly #fetch: FetchLike;

  constructor(options: ClientOptions) {
    this.#endpoint = options.endpoint;
    this.#fetch = options.fetch ?? ((input, init) => globalThis.fetch(input, init));
  }

  /** `GET /health`, which needs no token. */
  async health(): Promise<Health> {
    return this.#request("GET", "/health", HealthSchema, undefined, false);
  }

  /** `POST /predict`. */
  async predict(
    texts: readonly string[],
    options: { topK?: number; creatorId?: string; sourceId?: string } = {},
  ): Promise<Prediction[]> {
    const body: Record<string, unknown> = { texts: [...texts] };
    if (options.topK !== undefined) body["top_k"] = options.topK;
    if (options.creatorId !== undefined) body["creator_id"] = options.creatorId;
    if (options.sourceId !== undefined) body["source_id"] = options.sourceId;

    const response = await this.#request("POST", "/predict", PredictResponseSchema, body);
    return response.predictions;
  }

  /** `POST /parse`. */
  async parse(texts: readonly string[]): Promise<SlateFields[]> {
    const response = await this.#request("POST", "/parse", ParseResponseSchema, {
      texts: [...texts],
    });
    return response.slates;
  }

  /** `GET /taxonomy/search`. */
  async searchTaxonomy(query: string, limit?: number): Promise<TaxonomyEntry[]> {
    const parameters = new URLSearchParams({ q: query });
    if (limit !== undefined) parameters.set("limit", String(limit));
    const response = await this.#request(
      "GET",
      `/taxonomy/search?${parameters.toString()}`,
      TaxonomySearchResponseSchema,
    );
    return response.entries;
  }

  /** `GET /taxonomy/{cat_id}`. */
  async taxonomyEntry(catId: string): Promise<TaxonomyEntry> {
    const response = await this.#request(
      "GET",
      `/taxonomy/${encodeURIComponent(catId)}`,
      TaxonomyEntryResponseSchema,
    );
    return response.entry;
  }

  /** `POST /shutdown`. */
  async shutdown(): Promise<boolean> {
    const response = await this.#request("POST", "/shutdown", ShutdownResponseSchema, {});
    return response.ok;
  }

  async #request<T>(
    method: "GET" | "POST",
    path: string,
    schema: ZodType<T>,
    body?: unknown,
    authenticated = true,
  ): Promise<T> {
    const headers: Record<string, string> = { accept: "application/json" };
    if (authenticated) headers["x-ttucs-token"] = this.#endpoint.token;
    if (body !== undefined) headers["content-type"] = "application/json";

    let response: Response;
    try {
      response = await this.#fetch(`${this.#endpoint.baseUrl}${path}`, {
        method,
        headers,
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      });
    } catch (cause) {
      throw new EngineError(
        "transport",
        `could not reach the engine at ${this.#endpoint.baseUrl}: ${String(cause)}`,
      );
    }

    if (!response.ok) {
      throw new EngineError(
        kindForStatus(response.status),
        `engine returned ${response.status} for ${method} ${path}`,
        response.status,
      );
    }

    let payload: unknown;
    try {
      payload = await response.json();
    } catch (cause) {
      throw new EngineError("schema", `response was not JSON: ${String(cause)}`, response.status);
    }

    const parsed = schema.safeParse(payload);
    if (!parsed.success) {
      throw new EngineError(
        "schema",
        `response did not match the contract: ${parsed.error.issues
          .map((issue) => `${issue.path.join(".") || "<root>"} ${issue.message}`)
          .join("; ")}`,
        response.status,
      );
    }

    const version = (payload as { schema_version?: unknown }).schema_version;
    if (typeof version === "number" && !isSupportedSchemaVersion(version)) {
      throw new EngineError(
        "schema",
        `engine speaks schema_version ${version}, this client supports 1`,
        response.status,
      );
    }

    return parsed.data;
  }
}
