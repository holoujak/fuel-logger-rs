import { useEffect, useState } from "react";
import { fetchLogs, fetchUsers, Log, User } from "../api";

export default function LogsPage() {
    const [logs, setLogs] = useState<Log[]>([]);
    const [users, setUsers] = useState<Map<number, string>>(new Map());
    const [filterStation, setFilterStation] = useState<string>("");
    const [filterUser, setFilterUser] = useState<string>("");
    const [lightboxSrc, setLightboxSrc] = useState<string | null>(null);

    useEffect(() => {
        fetchUsers()
            .then((data) => {
                const map = new Map<number, string>();
                data.forEach((u: User) => map.set(u.id, u.name));
                setUsers(map);
            })
            .catch(console.error);
    }, []);

    useEffect(() => {
        const params: Record<string, number> = {};
        if (filterStation) params.station = Number(filterStation);
        if (filterUser) params.user_id = Number(filterUser);
        fetchLogs({ ...params, limit: 200 })
            .then(setLogs)
            .catch(console.error);
    }, [filterStation, filterUser]);

    const formatDate = (iso: string) =>
        new Date(iso).toLocaleString("cs-CZ");

    const formatLength = (secs: number) => {
        const m = Math.floor(secs / 60);
        const s = secs % 60;
        return `${m}m ${s}s`;
    };

    return (
        <div>
            <div className="toolbar">
                <h2 className="page-title">Fueling Logs</h2>
                <div className="form-inline">
                    <div className="form-group">
                        <label>Station</label>
                        <select
                            value={filterStation}
                            onChange={(e) => setFilterStation(e.target.value)}
                        >
                            <option value="">All</option>
                            <option value="1">S1</option>
                            <option value="2">S2</option>
                        </select>
                    </div>
                    <div className="form-group">
                        <label>User (ID)</label>
                        <input
                            value={filterUser}
                            onChange={(e) => setFilterUser(e.target.value)}
                            placeholder="ID"
                            style={{ width: 80 }}
                        />
                    </div>
                </div>
            </div>

            <table>
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Date</th>
                        <th>User</th>
                        <th>Station</th>
                        <th>Duration</th>
                        <th>Consumption (l)</th>
                        <th>Photo</th>
                    </tr>
                </thead>
                <tbody>
                    {logs.map((l) => (
                        <tr key={l.id}>
                            <td>{l.id}</td>
                            <td>{formatDate(l.created_at)}</td>
                            <td>{users.get(l.user_id) ?? l.user_id}</td>
                            <td>S{l.station}</td>
                            <td>{formatLength(l.length)}</td>
                            <td>{l.consumption.toFixed(2)}</td>
                            <td>
                                {l.snapshot_path && (
                                    <button
                                        className="btn-snapshot"
                                        title="View photo"
                                        onClick={() => setLightboxSrc(`/api/snapshots/${l.snapshot_path}`)}
                                    >
                                        📷
                                    </button>
                                )}
                            </td>
                        </tr>
                    ))}
                    {logs.length === 0 && (
                        <tr>
                            <td colSpan={7} style={{ textAlign: "center" }}>
                                No records found.
                            </td>
                        </tr>
                    )}
                </tbody>
            </table>

            {lightboxSrc && (
                <div className="modal-overlay" onClick={() => setLightboxSrc(null)}>
                    <div className="lightbox" onClick={(e) => e.stopPropagation()}>
                        <img src={lightboxSrc} alt="Snapshot" />
                        <button className="lightbox-close" onClick={() => setLightboxSrc(null)}>✕</button>
                    </div>
                </div>
            )}
        </div>
    );
}
