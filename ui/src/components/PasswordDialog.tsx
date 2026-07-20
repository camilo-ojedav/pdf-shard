import { useState } from "react";
import { S } from "../strings/es";
import { fileTitle, useDocuments } from "../state/document";

export function PasswordDialog() {
  const prompt = useDocuments((s) => s.passwordPrompt);
  const openPath = useDocuments((s) => s.openPath);
  const cancel = useDocuments((s) => s.cancelPasswordPrompt);
  const [password, setPassword] = useState("");

  if (!prompt) return null;

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!prompt) return;
    void openPath(prompt.path, password).then(() => setPassword(""));
  }

  return (
    <div className="modal-backdrop">
      <form className="modal" onSubmit={onSubmit}>
        <h2>{S.password.titulo}</h2>
        <p className="modal-file" title={prompt.path}>
          {fileTitle(prompt.path)}
        </p>
        <p>{prompt.kind === "wrong_password" ? S.password.incorrecta : S.password.pideContrasena}</p>
        <input
          type="password"
          autoFocus
          placeholder={S.password.placeholder}
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
        <div className="modal-actions">
          <button
            type="button"
            onClick={() => {
              setPassword("");
              cancel();
            }}
          >
            {S.password.cancelar}
          </button>
          <button type="submit" disabled={password.length === 0}>
            {S.password.abrir}
          </button>
        </div>
      </form>
    </div>
  );
}
