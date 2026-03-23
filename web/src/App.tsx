import { Routes, Route, Link } from "react-router-dom";
import UsersPage from "./pages/UsersPage";
import StationsPage from "./pages/StationsPage";
import LogsPage from "./pages/LogsPage";
import StatsPage from "./pages/StatsPage";

export default function App() {
    return (
        <div className="app">
            <nav className="navbar">
                <h1>⛽ Fuel Logger</h1>
                <div className="nav-links">
                    <Link to="/">Stations</Link>
                    <Link to="/users">Users</Link>
                    <Link to="/logs">Logs</Link>
                    <Link to="/stats">Statistics</Link>
                </div>
            </nav>
            <main className="content">
                <Routes>
                    <Route path="/" element={<StationsPage />} />
                    <Route path="/users" element={<UsersPage />} />
                    <Route path="/logs" element={<LogsPage />} />
                    <Route path="/stats" element={<StatsPage />} />
                </Routes>
            </main>
        </div>
    );
}
