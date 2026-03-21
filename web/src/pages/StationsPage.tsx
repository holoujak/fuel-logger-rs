import { useEffect, useState } from "react";
import { fetchStations, StationInfo } from "../api";

export default function StationsPage() {
    const [stations, setStations] = useState<StationInfo[]>([]);

    useEffect(() => {
        const load = () => fetchStations().then(setStations).catch(console.error);
        load();
        const interval = setInterval(load, 2000);
        return () => clearInterval(interval);
    }, []);

    const formatTime = (secs: number | null) => {
        if (secs == null) return "–";
        const m = Math.floor(secs / 60);
        const s = secs % 60;
        return `${m}m ${s}s`;
    };

    return (
        <div>
            <h2 className="page-title" style={{ marginBottom: "1.5rem" }}>
                Stations
            </h2>
            <div className="stations-grid">
                {stations.map((st) => (
                    <div key={st.id} className="station-card">
                        <h2>
                            {st.name}{" "}
                            <span className={`status status-${st.status}`}>{st.status}</span>
                        </h2>
                        <div className="info-row">
                            <span>User</span>
                            <span>{st.active_user ?? "–"}</span>
                        </div>
                        <div className="info-row">
                            <span>Pumping time</span>
                            <span>{formatTime(st.current_length_secs)}</span>
                        </div>
                        <div className="info-row">
                            <span>Flow meter pulses</span>
                            <span>{st.pulses_count}</span>
                        </div>
                    </div>
                ))}
                {stations.length === 0 && <p>No stations found.</p>}
            </div>
        </div>
    );
}
