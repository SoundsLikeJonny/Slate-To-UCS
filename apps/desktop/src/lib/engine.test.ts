import { describe, expect, it, vi } from "vitest";

import { EngineClient, EngineError, type FetchLike } from "./engine.ts";

const endpoint = { baseUrl: "http://127.0.0.1:8731", token: "s3cret" };

const health = {
  schema_version: 1,
  status: "ok",
  warm: true,
  ttucs_version: "0.3.0",
  ucs_version: "8.2.1",
  model: { name: "bow", version: "0.2.0" },
  offline: true,
  unknown_threshold: 0.4,
  category_threshold: 0.55,
};

const prediction = {
  cat_id: "DOORCreak",
  category: "DOOR",
  subcategory: "Creak",
  cat_short: "DOORCreak",
  category_full: "Doors",
  confidence: 0.91,
  decision: "confident",
  filename: "DOORCreak_Door Creak_NONE_NONE",
};

/** A fetch stub that records calls and answers with one JSON body. */
function stub(body: unknown, status = 200): { fetch: FetchLike; calls: Array<[string, RequestInit | undefined]> } {
  const calls: Array<[string, RequestInit | undefined]> = [];
  const fetch: FetchLike = (input, init) => {
    calls.push([input, init]);
    return Promise.resolve(
      new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
      }),
    );
  };
  return { fetch, calls };
}

describe("EngineClient.health", () => {
  it("calls /health without a token", async () => {
    const { fetch, calls } = stub(health);
    const client = new EngineClient({ endpoint, fetch });

    const result = await client.health();

    expect(result.ttucs_version).toBe("0.3.0");
    expect(calls[0]?.[0]).toBe("http://127.0.0.1:8731/health");
    const headers = calls[0]?.[1]?.headers as Record<string, string>;
    expect(headers["x-ttucs-token"]).toBeUndefined();
  });
});

describe("EngineClient.predict", () => {
  it("posts the texts and returns the predictions", async () => {
    const { fetch, calls } = stub({ schema_version: 1, predictions: [prediction] });
    const client = new EngineClient({ endpoint, fetch });

    const result = await client.predict(["door creak"]);

    expect(result).toHaveLength(1);
    expect(result[0]?.cat_id).toBe("DOORCreak");
    expect(calls[0]?.[1]?.method).toBe("POST");
    expect(JSON.parse(String(calls[0]?.[1]?.body))).toEqual({ texts: ["door creak"] });
  });

  it("sends the token on an authenticated route", async () => {
    const { fetch, calls } = stub({ schema_version: 1, predictions: [] });
    const client = new EngineClient({ endpoint, fetch });

    await client.predict(["x"]);

    const headers = calls[0]?.[1]?.headers as Record<string, string>;
    expect(headers["x-ttucs-token"]).toBe("s3cret");
  });

  it("includes top_k, creator and source when given", async () => {
    const { fetch, calls } = stub({ schema_version: 1, predictions: [] });
    const client = new EngineClient({ endpoint, fetch });

    await client.predict(["x"], { topK: 5, creatorId: "JON", sourceId: "LIB" });

    expect(JSON.parse(String(calls[0]?.[1]?.body))).toEqual({
      texts: ["x"],
      top_k: 5,
      creator_id: "JON",
      source_id: "LIB",
    });
  });

  it("omits optional fields that were not given", async () => {
    const { fetch, calls } = stub({ schema_version: 1, predictions: [] });
    const client = new EngineClient({ endpoint, fetch });

    await client.predict(["x"], { topK: 3 });

    const body = JSON.parse(String(calls[0]?.[1]?.body)) as Record<string, unknown>;
    expect(Object.keys(body).sort()).toEqual(["texts", "top_k"]);
  });
});

describe("EngineClient.parse and taxonomy", () => {
  it("returns the slates from /parse", async () => {
    const { fetch } = stub({
      schema_version: 1,
      slates: [{ description: "door creak", fx_name: "Door Creak", keywords: ["Door"] }],
    });
    const client = new EngineClient({ endpoint, fetch });

    const slates = await client.parse(["door creak"]);

    expect(slates[0]?.fx_name).toBe("Door Creak");
  });

  it("encodes the search query and limit", async () => {
    const { fetch, calls } = stub({ schema_version: 1, entries: [] });
    const client = new EngineClient({ endpoint, fetch });

    await client.searchTaxonomy("door slam", 5);

    expect(calls[0]?.[0]).toBe("http://127.0.0.1:8731/taxonomy/search?q=door+slam&limit=5");
  });

  it("omits the limit when it was not given", async () => {
    const { fetch, calls } = stub({ schema_version: 1, entries: [] });
    const client = new EngineClient({ endpoint, fetch });

    await client.searchTaxonomy("door");

    expect(calls[0]?.[0]).toBe("http://127.0.0.1:8731/taxonomy/search?q=door");
  });

  it("escapes a CatID in the path", async () => {
    const { fetch, calls } = stub({
      schema_version: 1,
      entry: {
        cat_id: "DOORCreak",
        category: "DOOR",
        subcategory: "Creak",
        cat_short: "DOORCreak",
        category_full: "Doors",
      },
    });
    const client = new EngineClient({ endpoint, fetch });

    const entry = await client.taxonomyEntry("DOOR Creak");

    expect(entry.cat_id).toBe("DOORCreak");
    expect(calls[0]?.[0]).toBe("http://127.0.0.1:8731/taxonomy/DOOR%20Creak");
  });
});

describe("EngineClient.shutdown", () => {
  it("returns the ok flag", async () => {
    const { fetch } = stub({ schema_version: 1, ok: true });
    const client = new EngineClient({ endpoint, fetch });

    await expect(client.shutdown()).resolves.toBe(true);
  });
});

describe("EngineClient failures", () => {
  it("maps 401 to unauthorised", async () => {
    const { fetch } = stub({ error: "no" }, 401);
    const client = new EngineClient({ endpoint, fetch });

    await expect(client.predict(["x"])).rejects.toMatchObject({
      kind: "unauthorised",
      status: 401,
    });
  });

  it("maps 413 to too_large", async () => {
    const { fetch } = stub({ error: "big" }, 413);
    const client = new EngineClient({ endpoint, fetch });

    await expect(client.predict(["x"])).rejects.toMatchObject({ kind: "too_large" });
  });

  it("maps 400 to bad_request", async () => {
    const { fetch } = stub({ error: "bad" }, 400);
    const client = new EngineClient({ endpoint, fetch });

    await expect(client.predict(["x"])).rejects.toMatchObject({ kind: "bad_request" });
  });

  it("maps 404 to not_found", async () => {
    const { fetch } = stub({ error: "gone" }, 404);
    const client = new EngineClient({ endpoint, fetch });

    await expect(client.taxonomyEntry("NOPE")).rejects.toMatchObject({ kind: "not_found" });
  });

  it("maps 500 to status and keeps the code", async () => {
    const { fetch } = stub({ error: "boom" }, 500);
    const client = new EngineClient({ endpoint, fetch });

    await expect(client.health()).rejects.toMatchObject({ kind: "status", status: 500 });
  });

  it("reports a transport failure with the endpoint", async () => {
    const fetch: FetchLike = () => Promise.reject(new Error("connection refused"));
    const client = new EngineClient({ endpoint, fetch });

    const error = await client.health().catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(EngineError);
    expect((error as EngineError).kind).toBe("transport");
    expect((error as EngineError).message).toContain("127.0.0.1:8731");
  });

  it("rejects a body that is not JSON", async () => {
    const fetch: FetchLike = () =>
      Promise.resolve(new Response("not json", { status: 200 }));
    const client = new EngineClient({ endpoint, fetch });

    await expect(client.health()).rejects.toMatchObject({ kind: "schema" });
  });

  it("rejects a body that does not match the contract and names the field", async () => {
    const { fetch } = stub({ schema_version: 1, predictions: [{ cat_id: 1 }] });
    const client = new EngineClient({ endpoint, fetch });

    const error = (await client.predict(["x"]).catch((caught: unknown) => caught)) as EngineError;

    expect(error.kind).toBe("schema");
    expect(error.message).toContain("predictions.0.cat_id");
  });

  it("rejects an unsupported schema version", async () => {
    const { fetch } = stub({ ...health, schema_version: 2 });
    const client = new EngineClient({ endpoint, fetch });

    const error = (await client.health().catch((caught: unknown) => caught)) as EngineError;

    expect(error.kind).toBe("schema");
    expect(error.message).toContain("schema_version 2");
  });

  it("uses the global fetch when none is injected", async () => {
    const spy = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(new Response(JSON.stringify(health), { status: 200 }));
    const client = new EngineClient({ endpoint });

    await client.health();

    expect(spy).toHaveBeenCalledOnce();
    spy.mockRestore();
  });
});
