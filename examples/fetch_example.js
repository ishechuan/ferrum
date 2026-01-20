/// Example: Fetch API Usage
///
/// This example demonstrates the use of Deno.fetch() for making HTTP requests.
/// Run with: ferrum run --allow-net examples/fetch_example.js

// Simple GET request
console.log("=== Simple GET Request ===");
const response1 = Deno.fetch("https://example.com");
console.log("Status:", response1.status, response1.statusText);
console.log("OK:", response1.ok);
console.log("URL:", response1.url);
console.log("Headers:", response1.headers);

// Get response as text
const text = response1.text();
console.log("Body length:", text.length);
console.log("Body preview:", text.substring(0, 100));

console.log("\n=== JSON Request ===");
// Fetch JSON data
const response2 = Deno.fetch("https://httpbin.org/json");
console.log("JSON Status:", response2.status);
const jsonData = response2.json();
console.log("JSON data:", JSON.stringify(jsonData, null, 2));

console.log("\n=== POST Request with Headers ===");
// POST request with custom headers
const response3 = Deno.fetch("https://httpbin.org/post", {
    method: "POST",
    headers: {
        "Content-Type": "application/json",
        "X-Custom-Header": "Ferrum-Test"
    },
    body: JSON.stringify({ message: "Hello from Ferrum!" })
});
console.log("POST Status:", response3.status);
const postResponse = response3.json();
console.log("POST response data:", JSON.stringify(postResponse, null, 2));

console.log("\n=== Fetch with Query Parameters ===");
const response4 = Deno.fetch("https://httpbin.org/get?foo=bar&baz=qux");
console.log("Query Params Status:", response4.status);
const queryResponse = response4.json();
console.log("Query args:", JSON.stringify(queryResponse.args, null, 2));

console.log("\n=== All fetch examples completed successfully! ===");
