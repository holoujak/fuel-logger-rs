import { useEffect, useState } from "react";
import {
    fetchUsers,
    createUser,
    updateUser,
    deleteUser,
    User,
    CreateUser,
} from "../api";

function UserModal({
    user,
    onClose,
    onSave,
}: {
    user: User | null;
    onClose: () => void;
    onSave: (data: CreateUser) => void;
}) {
    const [name, setName] = useState(user?.name ?? "");
    const [tag, setTag] = useState(user?.tag ?? "");
    const [station1, setStation1] = useState(user?.station1 ?? false);
    const [station2, setStation2] = useState(user?.station2 ?? false);

    return (
        <div className="modal-overlay" onClick={onClose}>
            <div className="modal" onClick={(e) => e.stopPropagation()}>
                <h2>{user ? "Edit User" : "New User"}</h2>
                <div className="form-group">
                    <label>Name</label>
                    <input value={name} onChange={(e) => setName(e.target.value)} />
                </div>
                <div className="form-group">
                    <label>Tag (card code / password)</label>
                    <input value={tag} onChange={(e) => setTag(e.target.value)} />
                </div>
                <div className="form-group">
                    <label>
                        <input
                            type="checkbox"
                            checked={station1}
                            onChange={(e) => setStation1(e.target.checked)}
                        />{" "}
                        Station 1
                    </label>
                </div>
                <div className="form-group">
                    <label>
                        <input
                            type="checkbox"
                            checked={station2}
                            onChange={(e) => setStation2(e.target.checked)}
                        />{" "}
                        Station 2
                    </label>
                </div>
                <div className="modal-actions">
                    <button className="btn-secondary" onClick={onClose}>
                        Cancel
                    </button>
                    <button
                        className="btn-primary"
                        onClick={() => onSave({ name, tag, station1, station2 })}
                    >
                        Save
                    </button>
                </div>
            </div>
        </div>
    );
}

export default function UsersPage() {
    const [users, setUsers] = useState<User[]>([]);
    const [editing, setEditing] = useState<User | null | "new">(null);

    const load = () => fetchUsers().then(setUsers).catch(console.error);
    useEffect(() => {
        load();
    }, []);

    const handleSave = async (data: CreateUser) => {
        if (editing === "new") {
            await createUser(data);
        } else if (editing) {
            await updateUser(editing.id, data);
        }
        setEditing(null);
        load();
    };

    const handleDelete = async (id: number) => {
        if (!confirm("Are you sure you want to delete this user?")) return;
        await deleteUser(id);
        load();
    };

    return (
        <div>
            <div className="toolbar">
                <h2 className="page-title">Users</h2>
                <button className="btn-primary" onClick={() => setEditing("new")}>
                    + Add
                </button>
            </div>

            <table>
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Name</th>
                        <th>Tag</th>
                        <th>S1</th>
                        <th>S2</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {users.map((u) => (
                        <tr key={u.id}>
                            <td>{u.id}</td>
                            <td>{u.name}</td>
                            <td>{u.tag}</td>
                            <td>{u.station1 ? "✅" : "❌"}</td>
                            <td>{u.station2 ? "✅" : "❌"}</td>
                            <td>
                                <button
                                    className="btn-secondary"
                                    style={{ marginRight: "0.5rem" }}
                                    onClick={() => setEditing(u)}
                                >
                                    Edit
                                </button>
                                <button
                                    className="btn-danger"
                                    onClick={() => handleDelete(u.id)}
                                >
                                    Delete
                                </button>
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>

            {editing !== null && (
                <UserModal
                    user={editing === "new" ? null : editing}
                    onClose={() => setEditing(null)}
                    onSave={handleSave}
                />
            )}
        </div>
    );
}
