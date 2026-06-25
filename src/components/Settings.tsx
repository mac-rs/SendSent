import { useEffect, useState } from "react";
import { getIdentity, setDisplayName } from "../lib/invoke";

export function Settings() {
  const [name, setName] = useState("");
  useEffect(() => { getIdentity().then((i) => setName(i.name)); }, []);
  return (
    <div style={{ marginTop: 16 }}>
      <input value={name} onChange={(e) => setName(e.target.value)} />
      <button onClick={() => setDisplayName(name)}>保存名字(重启生效)</button>
    </div>
  );
}
