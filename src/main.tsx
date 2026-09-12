import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";
import "./theme.css";
// 5 个平台各自一套 CSS;运行时由 .app.<platform> 选择器激活
import "./platforms/macos/macos.css";
import "./platforms/ios/ios.css";
import "./platforms/android/android.css";
import "./platforms/windows/windows.css";
import "./platforms/linux/linux.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode><App /></React.StrictMode>,
);
