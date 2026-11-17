export interface paths {
    "/api/v1/agents": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["agents"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/agents/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["agent_detail"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/agents/{id}/files": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["agent_files"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/agents/{id}/logs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["agent_logs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/agents/{id}/sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["agent_sessions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/agents/{id}/tasks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["agent_tasks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/agents/{id}/usage": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["agent_usage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/alerts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["alerts"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/alerts/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["alert_detail"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/alerts/{id}/ack": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["ack_alert"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/alerts/{id}/resolve": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["resolve_alert"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/alerts/{id}/snooze": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["snooze_alert"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/pair": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["pair"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/ws-ticket": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["ws_ticket"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/costs": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["costs"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/devices": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["devices"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/devices/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["revoke_device"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/openapi.json": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["openapi_json"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/overview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["overview"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tasks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["tasks"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tasks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["task_detail"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/timeline": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["timeline"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/usage": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["usage"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        AgentCardDto: {
            id: string;
            name: string;
            status: string;
        };
        AgentDetailDto: {
            /** Format: int64 */
            as_of?: number | null;
            card: components["schemas"]["AgentCardDto"];
            sessions: components["schemas"]["SessionView"][];
            tasks: components["schemas"]["TaskView"][];
        };
        AgentPage: {
            items: components["schemas"]["AgentCardDto"][];
            next_cursor?: string | null;
        };
        AlertActionResult: {
            id: string;
            /** Format: int64 */
            seq: number;
            /** Format: int64 */
            snooze_until_ms?: number | null;
            state: string;
        };
        AlertMutation: {
            /** Format: int64 */
            minutes?: number | null;
            note?: string | null;
        };
        AlertView: {
            /** Format: int64 */
            ack_ms?: number | null;
            fingerprint: string;
            id: string;
            level: string;
            /** Format: int64 */
            occurrences: number;
            /** Format: int64 */
            opened_ms: number;
            /** Format: int64 */
            resolved_ms?: number | null;
            rule_id: string;
            /** Format: int64 */
            snooze_until_ms?: number | null;
            state: string;
            subject: string;
            title: string;
            /** Format: int64 */
            updated_ms: number;
        };
        ChangedFile: {
            agent_id: string;
            /** Format: int64 */
            changed_ms: number;
            path: string;
        };
        CostReport: {
            /** Format: double */
            cash_usd: number;
            /** Format: double */
            shadow_usd: number;
        };
        Counts: {
            /** Format: int64 */
            alerts_open: number;
            /** Format: int64 */
            offline: number;
            /** Format: int64 */
            waiting: number;
            /** Format: int64 */
            working: number;
        };
        DbHealth: {
            journal_mode: string;
            /** Format: int64 */
            migrations: number;
            ok: boolean;
            tables: number;
            /** Format: int64 */
            wal_size_bytes: number;
        };
        DeviceView: {
            /** Format: int64 */
            created_ms: number;
            /** Format: int64 */
            expires_ms?: number | null;
            id: string;
            name: string;
            /** Format: int64 */
            revoked_ms?: number | null;
            scopes: string[];
        };
        ErrorBody: {
            code: string;
            detail?: unknown;
            message: string;
            request_id?: string | null;
        };
        EventPage: {
            items: components["schemas"]["EventView"][];
            next_cursor?: string | null;
        };
        EventView: {
            /** Format: int64 */
            event_ts_ms: number;
            kind: string;
            payload: unknown;
            /** Format: int64 */
            seq: number;
            source: string;
            subject: string;
        };
        HealthResponse: {
            db: components["schemas"]["DbHealth"];
            integrity_ok: boolean;
            /** Format: int64 */
            last_seq: number;
            ok: boolean;
            /** Format: int64 */
            seq: number;
            /** Format: int64 */
            uptime_secs: number;
        };
        LogPage: {
            items: components["schemas"]["LogView"][];
            next_cursor?: string | null;
        };
        LogView: {
            id: string;
            level: string;
            text: string;
            /** Format: int64 */
            ts_ms: number;
        };
        Overview: {
            agents: components["schemas"]["AgentCardDto"][];
            counts: components["schemas"]["Counts"];
            /** Format: int64 */
            seq: number;
            /** Format: int64 */
            usage_requests: number;
        };
        PairInput: {
            code: string;
            name?: string | null;
        };
        PairResponse: {
            device_id: string;
            /** Format: int64 */
            expires_ms?: number | null;
            token: string;
        };
        SessionPage: {
            items: components["schemas"]["SessionView"][];
            next_cursor?: string | null;
        };
        SessionView: {
            agent_id: string;
            /** Format: double */
            cash_usd: number;
            /** Format: int64 */
            ended_ms?: number | null;
            id: string;
            /** Format: int64 */
            input_tokens: number;
            outcome?: string | null;
            /** Format: int64 */
            output_tokens: number;
            /** Format: int64 */
            requests: number;
            runtime: string;
            /** Format: int64 */
            started_ms: number;
        };
        TaskPage: {
            items: components["schemas"]["TaskView"][];
            next_cursor?: string | null;
        };
        TaskView: {
            agent_id: string;
            /** Format: int64 */
            ended_ms?: number | null;
            id: string;
            phase: string;
            session_id?: string | null;
            /** Format: int64 */
            started_ms: number;
            title: string;
            /** Format: int64 */
            updated_ms: number;
        };
        UsageReportDto: {
            total: components["schemas"]["UsageTotalsDto"];
        };
        UsageTotalsDto: {
            /** Format: int64 */
            cache_tokens: number;
            /** Format: double */
            cash_usd: number;
            /** Format: int64 */
            input_tokens: number;
            /** Format: int64 */
            output_tokens: number;
            /** Format: int64 */
            requests: number;
            /** Format: double */
            shadow_usd: number;
        };
        WsTicketResponse: {
            /** Format: int64 */
            expires_ms: number;
            ticket: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    agents: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                status: string | null;
                kind: string | null;
                q: string | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Agent cards */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentPage"];
                };
            };
        };
    };
    agent_detail: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Stable agent id */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Agent detail */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AgentDetailDto"];
                };
            };
        };
    };
    agent_files: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Stable agent id */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Changed files */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ChangedFile"][];
                };
            };
        };
    };
    agent_logs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Stable agent id */
                id: string;
                level: string | null;
                q: string | null;
                limit: number | null;
                before: number | null;
                locale: string | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Rendered log page */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LogPage"];
                };
            };
        };
    };
    agent_sessions: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Stable agent id */
                id: string;
                limit: number | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Agent sessions */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SessionPage"];
                };
            };
        };
    };
    agent_tasks: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Stable agent id */
                id: string;
                limit: number | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Agent tasks */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TaskPage"];
                };
            };
        };
    };
    agent_usage: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Stable agent id */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Agent usage */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["UsageReportDto"];
                };
            };
        };
    };
    alerts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                state: string | null;
                level: string | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Alerts */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlertView"][];
                };
            };
        };
    };
    alert_detail: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Alert id */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Alert detail */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlertView"];
                };
            };
        };
    };
    ack_alert: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AlertMutation"];
            };
        };
        responses: {
            /** @description Alert acknowledged */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlertActionResult"];
                };
            };
        };
    };
    resolve_alert: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AlertMutation"];
            };
        };
        responses: {
            /** @description Alert resolved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlertActionResult"];
                };
            };
        };
    };
    snooze_alert: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AlertMutation"];
            };
        };
        responses: {
            /** @description Alert snoozed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlertActionResult"];
                };
            };
        };
    };
    pair: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PairInput"];
            };
        };
        responses: {
            /** @description Scoped device token */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PairResponse"];
                };
            };
        };
    };
    ws_ticket: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Single-use WS ticket */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["WsTicketResponse"];
                };
            };
        };
    };
    costs: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                agent: string | null;
                group_by: string | null;
                window: string | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Cost totals */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CostReport"];
                };
            };
        };
    };
    devices: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Devices */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["DeviceView"][];
                };
            };
        };
    };
    revoke_device: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Device revoked */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    health: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Health */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HealthResponse"];
                };
            };
        };
    };
    openapi_json: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OpenAPI document */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
        };
    };
    overview: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current operational snapshot */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Overview"];
                };
            };
        };
    };
    tasks: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                agent: string | null;
                phase: string | null;
                q: string | null;
                limit: number | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Task page */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TaskPage"];
                };
            };
        };
    };
    task_detail: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Task id */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Task detail */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["TaskView"];
                };
            };
        };
    };
    timeline: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Seq cursor. Both `since` and `cursor` are accepted. */
                since: number | null;
                cursor: number | null;
                until: number | null;
                kind: string | null;
                subject: string | null;
                limit: number | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Event page */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["EventPage"];
                };
            };
        };
    };
    usage: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                agent: string | null;
                group_by: string | null;
                window: string | null;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Usage totals */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["UsageReportDto"];
                };
            };
        };
    };
}
