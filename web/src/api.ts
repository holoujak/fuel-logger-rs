// ─── Types matching Rust API models ──────────────────────────────────────────

export interface User {
    id: number;
    name: string;
    tag: string;
    station1: boolean;
    station2: boolean;
}

export interface CreateUser {
    name: string;
    tag: string;
    station1: boolean;
    station2: boolean;
}

export interface StationInfo {
    id: number;
    name: string;
    status: string;
    current_length_secs: number | null;
    pulses_count: number;
    active_user: string | null;
}

export interface Log {
    id: number;
    user_id: number;
    created_at: string;
    station: number;
    length: number;
    consumption: number;
    snapshot_path: string | null;
}

export interface LogQuery {
    station?: number;
    user_id?: number;
    limit?: number;
    offset?: number;
}

// ─── API helpers ─────────────────────────────────────────────────────────────

const BASE = "/api";

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
    const res = await fetch(`${BASE}${path}`, {
        headers: { "Content-Type": "application/json" },
        ...init,
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(`API ${res.status}: ${text}`);
    }
    if (res.status === 204) return undefined as unknown as T;
    return res.json();
}

// Users
export const fetchUsers = () => apiFetch<User[]>("/users");
export const fetchUser = (id: number) => apiFetch<User>(`/users/${id}`);
export const createUser = (data: CreateUser) =>
    apiFetch<User>("/users", { method: "POST", body: JSON.stringify(data) });
export const updateUser = (id: number, data: Partial<CreateUser>) =>
    apiFetch<User>(`/users/${id}`, { method: "PUT", body: JSON.stringify(data) });
export const deleteUser = (id: number) =>
    apiFetch<void>(`/users/${id}`, { method: "DELETE" });

// Stations
export const fetchStations = () => apiFetch<StationInfo[]>("/stations");

// Logs
export const fetchLogs = (params?: LogQuery) => {
    const qs = new URLSearchParams();
    if (params?.station != null) qs.set("station", String(params.station));
    if (params?.user_id != null) qs.set("user_id", String(params.user_id));
    if (params?.limit != null) qs.set("limit", String(params.limit));
    if (params?.offset != null) qs.set("offset", String(params.offset));
    const q = qs.toString();
    return apiFetch<Log[]>(`/logs${q ? "?" + q : ""}`);
};
