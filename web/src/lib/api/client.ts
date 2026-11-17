/// <reference lib="es2020" />
/// <reference lib="dom" />
import type { paths } from "./schema";

type JsonBody<T> = T extends { content: { "application/json": infer Body } }
  ? Body
  : never;

export type Overview = JsonBody<
  paths["/api/v1/overview"]["get"]["responses"][200]
>;
export type AgentPage = JsonBody<
  paths["/api/v1/agents"]["get"]["responses"][200]
>;
export type AgentDetail = JsonBody<
  paths["/api/v1/agents/{id}"]["get"]["responses"][200]
>;
export type TaskPage = JsonBody<
  paths["/api/v1/tasks"]["get"]["responses"][200]
>;
export type LogPage = JsonBody<
  paths["/api/v1/agents/{id}/logs"]["get"]["responses"][200]
>;
export type AlertView = JsonBody<
  paths["/api/v1/alerts"]["get"]["responses"][200]
>[number];
export type AlertActionResult = JsonBody<
  paths["/api/v1/alerts/{id}/ack"]["post"]["responses"][200]
>;
export type EventPage = JsonBody<
  paths["/api/v1/timeline"]["get"]["responses"][200]
>;
export type UsageReport = JsonBody<
  paths["/api/v1/usage"]["get"]["responses"][200]
>;
export type CostReport = JsonBody<
  paths["/api/v1/costs"]["get"]["responses"][200]
>;

type ErrorBody = {
  code: string;
  message: string;
  detail: unknown;
  request_id: string | null;
};

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly detail: unknown;
  readonly requestId: string | null;

  constructor(status: number, body: ErrorBody) {
    super(body.message ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = body.code ?? "unknown";
    this.detail = body.detail ?? null;
    this.requestId = body.request_id ?? null;
  }
}

function csrfToken(): string {
  if (typeof document === "undefined") return "";
  const cookie = document.cookie
    .split(";")
    .map((part) => part.trim())
    .find((part) => part.startsWith("csrf="));
  return cookie ? decodeURIComponent(cookie.slice("csrf=".length)) : "";
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");
  if (init.method && init.method !== "GET") {
    headers.set("X-CSRF-Token", csrfToken());
  }
  const response = await fetch(`/api/v1${path}`, {
    ...init,
    headers,
    credentials: "same-origin",
  });
  if (!response.ok) {
    let body: ErrorBody;
    try {
      body = (await response.json()) as ErrorBody;
    } catch {
      body = {
        code: "http_error",
        message: `HTTP ${response.status}`,
        detail: null,
        request_id: null,
      };
    }
    throw new ApiError(response.status, body);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

function jsonBody(value: unknown): string {
  return JSON.stringify(value);
}

export function getOverview(): Promise<Overview> {
  return request("/overview");
}

export function getAgents(status?: string): Promise<AgentPage> {
  const query = status ? `?status=${encodeURIComponent(status)}` : "";
  return request(`/agents${query}`);
}

export function getAgent(id: string, asOf?: number): Promise<AgentDetail> {
  const query = asOf === undefined ? "" : `?as_of=${asOf}`;
  return request(`/agents/${encodeURIComponent(id)}${query}`);
}

export function getTasks(agent?: string): Promise<TaskPage> {
  const query = agent ? `?agent=${encodeURIComponent(agent)}` : "";
  return request(`/tasks${query}`);
}

export function getLogs(agentId: string, locale = "id"): Promise<LogPage> {
  return request(
    `/agents/${encodeURIComponent(agentId)}/logs?locale=${encodeURIComponent(locale)}`,
  );
}

export function getAlerts(state?: string): Promise<AlertView[]> {
  const query = state ? `?state=${encodeURIComponent(state)}` : "";
  return request(`/alerts${query}`);
}

export function ackAlert(id: string, note?: string): Promise<AlertActionResult> {
  return request(`/alerts/${encodeURIComponent(id)}/ack`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: jsonBody({ note }),
  });
}

export function snoozeAlert(
  id: string,
  minutes: number,
  note?: string,
): Promise<AlertActionResult> {
  return request(`/alerts/${encodeURIComponent(id)}/snooze`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: jsonBody({ minutes, note }),
  });
}

export function resolveAlert(
  id: string,
  note?: string,
): Promise<AlertActionResult> {
  return request(`/alerts/${encodeURIComponent(id)}/resolve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: jsonBody({ note }),
  });
}

export function getTimeline(since?: string, limit = 50): Promise<EventPage> {
  const params = new URLSearchParams({ limit: String(limit) });
  if (since) params.set("since", since);
  return request(`/timeline?${params.toString()}`);
}

export function getCosts(): Promise<CostReport> {
  return request("/costs");
}

export function getUsage(
  agent?: string,
  options?: { group_by?: string; window?: "24h" | "7d" }
): Promise<UsageReport> {
  const params = new URLSearchParams();
  if (agent) params.set("agent", agent);
  if (options?.group_by) params.set("group_by", options.group_by);
  if (options?.window) params.set("window", options.window);
  const query = params.toString();
  return request(`/usage${query ? `?${query}` : ""}`);
}
