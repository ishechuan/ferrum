#!/usr/bin/env ferrum
/**
 * HTTP Server Example
 *
 * This example demonstrates how to create a simple HTTP server in Ferrum.
 * Run with: cargo run -- run examples/http_server.js
 */

// Start HTTP server using Deno.serve()
const server = Deno.serve((req) => {
    const url = new URL(req.url, "http://localhost:8000");
    const path = url.pathname;

    console.log(`${req.method} ${path} from ${req.peerAddr || "unknown"}`);

    // Route handling
    if (path === "/" || path === "/index") {
        return {
            status: 200,
            headers: { "content-type": "text/html; charset=utf-8" },
            body: `<!DOCTYPE html>
<html>
<head>
    <title>Ferrum HTTP Server</title>
    <style>
        body { font-family: system-ui, sans-serif; max-width: 800px; margin: 2rem auto; padding: 1rem; }
        h1 { color: #333; }
        .info { background: #f0f0f0; padding: 1rem; border-radius: 4px; }
        code { background: #e0e0e0; padding: 0.2rem 0.4rem; border-radius: 2px; }
        ul { line-height: 1.6; }
    </style>
</head>
<body>
    <h1>Ferrum HTTP Server</h1>
    <div class="info">
        <p>Your Ferrum HTTP server is running!</p>
        <p>Request method: <code>${req.method}</code></p>
        <p>Request URL: <code>${req.url}</code></p>
    </div>
    <h2>Available endpoints:</h2>
    <ul>
        <li><a href="/hello">/hello</a> - Returns "Hello, World!"</li>
        <li><a href="/json">/json</a> - Returns JSON response</li>
        <li><a href="/time">/time</a> - Returns current time</li>
        <li><a href="/headers">/headers</a> - Lists request headers</li>
    </ul>
</body>
</html>`
        };
    }

    if (path === "/hello") {
        return {
            status: 200,
            headers: { "content-type": "text/plain; charset=utf-8" },
            body: "Hello, World!\n"
        };
    }

    if (path === "/json") {
        return {
            status: 200,
            headers: { "content-type": "application/json; charset=utf-8" },
            body: JSON.stringify({
                message: "Hello from Ferrum!",
                timestamp: Date.now(),
                version: "0.1.0",
                server: "Ferrum"
            }, null, 2)
        };
    }

    if (path === "/time") {
        const now = new Date();
        return {
            status: 200,
            headers: { "content-type": "application/json; charset=utf-8" },
            body: JSON.stringify({
                iso: now.toISOString(),
                unix: Math.floor(now.getTime() / 1000),
                timezone: Intl.DateTimeFormat().resolvedOptions().timeZone
            }, null, 2)
        };
    }

    if (path === "/headers") {
        let headersList = "";
        for (const [key, value] of Object.entries(req.headers)) {
            headersList += `${key}: ${value}\n`;
        }
        return {
            status: 200,
            headers: { "content-type": "text/plain; charset=utf-8" },
            body: `Request Headers:\n${headersList}`
        };
    }

    // 404 for unknown routes
    return {
        status: 404,
        headers: { "content-type": "text/plain; charset=utf-8" },
        body: "Not Found\n"
    };
}, { port: 8000, hostname: "0.0.0.0" });

// Log server address
console.log("HTTP Server started!");
console.log(`Listening on http://0.0.0.0:8000`);
console.log("Press Ctrl+C to stop the server");

// The server object has methods to control it:
// - server.addr() - Get the listening address
// - server.listening - Check if server is listening
// - server.close() - Close the server
