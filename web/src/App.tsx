import { Routes, Route, Link } from "react-router-dom";

export default function App() {
    return (
        <div className="app">
            <nav className="navbar">
                <h1>⛽ Fuel Logger</h1>
                <div className="nav-links">
                    <Link to="/">Stations</Link>
                </div>
            </nav>
            <main className="content">
                <Routes>
                    <Route path="/" element={<div>TODO</div>} />
                </Routes>
            </main>
        </div>
    );
}
