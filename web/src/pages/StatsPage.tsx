import { useEffect, useState } from "react";
import { fetchStats, UserStats } from "../api";

type Preset = "week" | "month" | "year" | "custom";

function startOfWeek(): string {
    const d = new Date();
    d.setDate(d.getDate() - d.getDay() + (d.getDay() === 0 ? -6 : 1)); // Monday
    return d.toISOString().slice(0, 10);
}

function startOfMonth(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`;
}

function startOfYear(): string {
    return `${new Date().getFullYear()}-01-01`;
}

function today(): string {
    return new Date().toISOString().slice(0, 10);
}

export default function StatsPage() {
    const [preset, setPreset] = useState<Preset>("month");
    const [from, setFrom] = useState(startOfMonth());
    const [to, setTo] = useState(today());
    const [station, setStation] = useState<string>("");
    const [stats, setStats] = useState<UserStats[]>([]);
    const [loading, setLoading] = useState(false);

    // Update date range when preset changes
    useEffect(() => {
        if (preset === "week") {
            setFrom(startOfWeek());
            setTo(today());
        } else if (preset === "month") {
            setFrom(startOfMonth());
            setTo(today());
        } else if (preset === "year") {
            setFrom(startOfYear());
            setTo(today());
        }
        // "custom" – keep whatever the user set
    }, [preset]);

    useEffect(() => {
        setLoading(true);
        fetchStats({
            from: from + "T00:00:00",
            to: to + "T23:59:59",
            station: station ? Number(station) : undefined,
        })
            .then(setStats)
            .catch(console.error)
            .finally(() => setLoading(false));
    }, [from, to, station]);

    const totalLiters = stats.reduce((s, r) => s + r.total_liters, 0);
    const totalRefuels = stats.reduce((s, r) => s + r.refuel_count, 0);

    const formatDuration = (secs: number) => {
        const h = Math.floor(secs / 3600);
        const m = Math.floor((secs % 3600) / 60);
        const s = secs % 60;
        if (h > 0) return `${h}h ${m}m ${s}s`;
        return `${m}m ${s}s`;
    };

    return (
        <div>
            <div className="toolbar">
                <h2 className="page-title">📊 Statistics</h2>
            </div>

            <div className="stats-filters">
                <div className="form-inline">
                    <div className="form-group">
                        <label>Period</label>
                        <select value={preset} onChange={(e) => setPreset(e.target.value as Preset)}>
                            <option value="week">This week</option>
                            <option value="month">This month</option>
                            <option value="year">This year</option>
                            <option value="custom">Custom</option>
                        </select>
                    </div>
                    <div className="form-group">
                        <label>From</label>
                        <input
                            type="date"
                            value={from}
                            onChange={(e) => {
                                setPreset("custom");
                                setFrom(e.target.value);
                            }}
                        />
                    </div>
                    <div className="form-group">
                        <label>To</label>
                        <input
                            type="date"
                            value={to}
                            onChange={(e) => {
                                setPreset("custom");
                                setTo(e.target.value);
                            }}
                        />
                    </div>
                    <div className="form-group">
                        <label>Station</label>
                        <select value={station} onChange={(e) => setStation(e.target.value)}>
                            <option value="">All</option>
                            <option value="1">S1</option>
                            <option value="2">S2</option>
                        </select>
                    </div>
                </div>
            </div>

            <div className="stats-summary">
                <div className="stats-card">
                    <span className="stats-card-value">{totalLiters.toFixed(1)} l</span>
                    <span className="stats-card-label">Total consumption</span>
                </div>
                <div className="stats-card">
                    <span className="stats-card-value">{totalRefuels}</span>
                    <span className="stats-card-label">Refueling count</span>
                </div>
                <div className="stats-card">
                    <span className="stats-card-value">{stats.length}</span>
                    <span className="stats-card-label">Active users</span>
                </div>
            </div>

            {loading ? (
                <p style={{ textAlign: "center", color: "var(--text-muted)" }}>Loading…</p>
            ) : (
                <table>
                    <thead>
                        <tr>
                            <th>#</th>
                            <th>User</th>
                            <th>Consumption (l)</th>
                            <th>Refuels</th>
                            <th>Total time</th>
                            <th>Avg per refuel (l)</th>
                        </tr>
                    </thead>
                    <tbody>
                        {stats.map((s, i) => (
                            <tr key={s.user_id}>
                                <td>{i + 1}</td>
                                <td>{s.user_name}</td>
                                <td>{s.total_liters.toFixed(2)}</td>
                                <td>{s.refuel_count}</td>
                                <td>{formatDuration(s.total_seconds)}</td>
                                <td>{(s.total_liters / s.refuel_count).toFixed(2)}</td>
                            </tr>
                        ))}
                        {stats.length === 0 && (
                            <tr>
                                <td colSpan={6} style={{ textAlign: "center" }}>
                                    No data for selected period.
                                </td>
                            </tr>
                        )}
                    </tbody>
                </table>
            )}
        </div>
    );
}
