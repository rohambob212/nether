import React from "react";
import ReactDOM from "react-dom/client";
import { MantineProvider, createTheme } from "@mantine/core";
import App from "./App";
import "@mantine/core/styles.css";
import "./styles.css";

const systemStack =
  'system-ui, "Segoe UI", Roboto, "Helvetica Neue", sans-serif';

const theme = createTheme({
  primaryColor: "grape",
  primaryShade: 5,
  defaultRadius: "md",
  fontFamily: systemStack,
  headings: { fontFamily: systemStack, fontWeight: "700" },
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <MantineProvider theme={theme} forceColorScheme="dark">
      <App />
    </MantineProvider>
  </React.StrictMode>
);
