// Target comes from the page URL hash or defaults to the LAN address of the spike host.
const target = "ws://10.0.2.2:7979";
const log = (s) => { document.getElementById("log").textContent += s + "\n"; };
document.addEventListener("securitypolicyviolation", (e) => log("csp-violation " + e.violatedDirective + " " + e.blockedURI));
log("origin " + location.origin + " secure=" + window.isSecureContext);
try {
  const ws = new WebSocket(target, ["curvyo-connector.1"]);
  ws.binaryType = "arraybuffer";
  ws.onopen = () => { log("open, protocol=" + ws.protocol); ws.send(new Uint8Array([1, 2, 3])); };
  ws.onmessage = (e) => {
    if (typeof e.data === "string") { log("text back"); return; }
    log("echo " + e.data.byteLength);
    ws.send("page-got-echo origin=" + location.origin + " secure=" + window.isSecureContext);
  };
  ws.onerror = () => log("error");
  ws.onclose = (e) => log("close " + e.code);
} catch (e) {
  log("throw " + e);
}
