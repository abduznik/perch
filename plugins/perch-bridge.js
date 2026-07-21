// perch-bridge.js — opencode plugin for Perch
// Subscribes to opencode events and forwards them to Perch's local HTTP ingest server.
//
// Version: 0.2.0

const INGEST_PORT = process.env.PERCH_INGEST_PORT || 4097;
const INGEST_URL = `http://127.0.0.1:${INGEST_PORT}/ingest`;

async function sendEvent(event, data) {
  try {
    await fetch(INGEST_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ event, data: data || null }),
    });
  } catch {
    // Perch not running or not reachable — ignore
  }
}

export const server = async (input) => {
  return {
    async event({ event }) {
      if (!event || !event.type) return;

      switch (event.type) {
        case "session.status": {
          const status = event.properties?.status;
          if (status?.type === "busy") {
            sendEvent("session.busy");
          } else if (status?.type === "idle") {
            sendEvent("session.idle");
          }
          break;
        }
        case "session.idle":
          sendEvent("session.idle");
          break;
        case "session.error":
          sendEvent("session.error", event.properties?.error?.data?.message);
          break;
        default:
          sendEvent(event.type, JSON.stringify(event.properties));
          break;
      }
    },
  };
};
