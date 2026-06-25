import { useState } from "react";
import { usePeers } from "./hooks/usePeers";
import { useTransfer } from "./hooks/useTransfer";
import { PeerList } from "./components/PeerList";
import { FilePicker } from "./components/FilePicker";
import { IncomingRequest } from "./components/IncomingRequest";
import { TransferProgress } from "./components/TransferProgress";
import { Settings } from "./components/Settings";
import type { Peer } from "./lib/types";
import "./App.css";

function App() {
  const peers = usePeers();
  const { request, progress } = useTransfer();
  const [selected, setSelected] = useState<Peer | null>(null);
  return (
    <main className="container">
      <h1>SendSent</h1>
      <PeerList peers={peers} onPick={setSelected} />
      <div style={{ marginTop: 12 }}><FilePicker peer={selected} /></div>
      <div style={{ marginTop: 12 }}><IncomingRequest req={request} /></div>
      <div style={{ marginTop: 12 }}><TransferProgress items={Object.values(progress)} /></div>
      <Settings />
    </main>
  );
}
export default App;
