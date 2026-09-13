import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";
import "./theme.css";
// 各平台各自一套 CSS;运行时由 .app.<platform> 选择器激活
// (iOS 已改为原生 SwiftUI,不再加载 Web)
import "./platforms/macos/macos.css";
import "./platforms/android/android.css";
import "./platforms/windows/windows.css";
import "./platforms/linux/linux.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode><App /></React.StrictMode>,
);
