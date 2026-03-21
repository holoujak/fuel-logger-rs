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
