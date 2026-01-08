#!/usr/bin/env ferrum
/**
 * Async/Await and Timers Example
 *
 * This example demonstrates asynchronous programming with timers in Ferrum.
 */

// Basic timeout
console.log("Starting...");

setTimeout(() => {
  console.log("This runs after 1 second");
}, 1000);

// Interval
let count = 0;
const intervalId = setInterval(() => {
  count++;
  console.log(`Interval tick ${count}`);
  if (count >= 5) {
    clearInterval(intervalId);
    console.log("Interval cleared");
  }
}, 500);

// Async/await with sleep
function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function example() {
  console.log("\nAsync example starting...");

  await sleep(1000);
  console.log("After 1 second");

  await sleep(1000);
  console.log("After 2 seconds");

  // Parallel execution
  console.log("\nParallel execution:");
  const [a, b, c] = await Promise.all([
    sleep(100).then(() => "First"),
    sleep(200).then(() => "Second"),
    sleep(50).then(() => "Third"),
  ]);
  console.log(a, b, c);

  // Sequential execution
  console.log("\nSequential execution:");
  const result1 = await sleep(100).then(() => "Step 1");
  console.log(result1);
  const result2 = await sleep(100).then(() => "Step 2");
  console.log(result2);
  const result3 = await sleep(100).then(() => "Step 3");
  console.log(result3);

  console.log("\nAsync example complete!");
}

// Error handling with async
async function errorExample() {
  try {
    await new Promise((_, reject) => {
      setTimeout(() => reject(new Error("Something went wrong!")), 100);
    });
  } catch (error) {
    console.log("\nCaught error:", error.message);
  }
}

// Run examples
example().then(() => {
  return errorExample();
}).then(() => {
  console.log("\nAll examples complete!");
});
