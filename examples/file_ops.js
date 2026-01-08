#!/usr/bin/env ferrum
/**
 * File Operations Example
 *
 * This example demonstrates file system operations in Ferrum.
 * Run with: ferrum run --allow-read --allow-write examples/file_ops.js
 */

// Read a text file
console.log("Reading example.txt...");
const content = Deno.readTextFileSync("./example.txt");
console.log("Content:", content);

// Write a text file
console.log("\nWriting to output.txt...");
Deno.writeTextFileSync("./output.txt", "Hello from Ferrum!\nThis is a new file.");

// Append to a file
console.log("Appending to output.txt...");
Deno.writeTextFileSync("./output.txt", "\nThis line was appended.", { append: true });

// Read the file again to verify
console.log("\nReading output.txt...");
const outputContent = Deno.readTextFileSync("./output.txt");
console.log(outputContent);

// Check if a file exists
const stat = Deno.statSync("./output.txt");
console.log("\nFile info:");
console.log("  Is file:", stat.isFile);
console.log("  Size:", stat.size);
console.log("  Modified:", new Date(stat.mtime * 1000));

// Create a directory
console.log("\nCreating directory...");
Deno.mkdirSync("./test_dir", { recursive: true });

// List directory contents
console.log("\nListing current directory:");
for (const entry of Deno.readDirSync(".")) {
  console.log(`  ${entry.name} (${entry.isDirectory ? "dir" : "file"})`);
}

// Clean up
console.log("\nCleaning up...");
Deno.removeSync("./output.txt");
Deno.removeSync("./test_dir", { recursive: true });

console.log("Done!");
