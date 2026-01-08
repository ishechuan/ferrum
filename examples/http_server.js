#!/usr/bin/env ferrum
/**
 * HTTP Server Example
 *
 * This example demonstrates how to create a simple HTTP server in Ferrum.
 * Run with: ferrum run --allow-net --allow-read examples/http_server.js
 */

// Simple HTTP request handler
async function handleRequest(req) {
  const url = new URL(req.url);
  const path = url.pathname;

  console.log(`${req.method} ${path}`);

  // Route handling
  if (path === "/") {
    return new Response(
      "Welcome to Ferrum HTTP Server!\n" +
      "Try /hello, /json, or /time\n",
      { status: 200, headers: { "content-type": "text/plain" } }
    );
  }

  if (path === "/hello") {
    return new Response(
      "Hello, World!\n",
      { status: 200, headers: { "content-type": "text/plain" } }
    );
  }

  if (path === "/json") {
    const data = {
      message: "Hello from Ferrum!",
      timestamp: Date.now(),
      version: "0.1.0"
    };
    return new Response(
      JSON.stringify(data, null, 2),
      { status: 200, headers: { "content-type": "application/json" } }
    );
  }

  if (path === "/time") {
    const now = new Date();
    return new Response(
      JSON.stringify({
        iso: now.toISOString(),
        unix: Math.floor(now.getTime() / 1000),
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone
      }),
      { status: 200, headers: { "content-type": "application/json" } }
    );
  }

  // 404 for unknown routes
  return new Response(
    "Not Found\n",
    { status: 404, headers: { "content-type": "text/plain" } }
  );
}

// Start server (placeholder - actual server implementation needed)
console.log("HTTP Server example");
console.log("Note: Full HTTP server implementation is pending");
console.log("\nTo run a real server, you would use:");
console.log("  Deno.serve(handleRequest)");
console.log("\nThis example demonstrates the API structure.");

// Simulate serving
const mockRequest = {
  method: "GET",
  url: "http://localhost:8000/hello"
};

handleRequest(mockRequest).then(response => {
  console.log("\nMock response:");
  console.log("Status:", response.status);
  console.log("Headers:", response.headers);
  response.text().then(text => console.log("Body:", text));
});
