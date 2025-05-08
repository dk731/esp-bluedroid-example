<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Connection status
let isConnected = ref(false);

// Control values
const isEnabled = ref(false);
const frequency = ref(1000);
const dutyCycle = ref(50);

// Throttling mechanism
let updateTimeout: number | null = null;
const THROTTLE_DELAY = 300; // 100ms throttle delay

// Listen for connection status events
listen<boolean>("connection-status", (event) => {
  console.log("Connection status event received: ", event.payload);
  isConnected.value = event.payload;
});

// Function to update LED configuration
const updateLedConfig = () => {
  if (!isConnected.value) return;

  invoke("update_led_config", {
    ledConfig: {
      pwm_duty: parseFloat(dutyCycle.value.toString()) / 100,
      pwm_frequency: parseFloat(frequency.value.toString()),
      enabled: isEnabled.value,
    },
  })
    .then((response) => {
      console.log("Control values updated:", response);
    })
    .catch((error) => {
      console.error("Error updating control values:", error);
    });
};

// Throttled update function
const throttledUpdate = () => {
  if (updateTimeout === null) {
    updateLedConfig();

    updateTimeout = window.setTimeout(() => {
      updateTimeout = null;
      // If values changed during throttle period, update again
      updateLedConfig();
    }, THROTTLE_DELAY);
  }
};

// Watch for changes and apply throttling
watch(
  [isEnabled, frequency, dutyCycle],
  () => {
    throttledUpdate();
  },
  { deep: true }
);
</script>

<template>
  <div class="app-container">
    <header>
      <h1>ESP-Bluedroid LED Controller</h1>
    </header>

    <main>
      <div class="status-card">
        <div class="status-indicator" :class="{ connected: isConnected }"></div>
        <p>Status: {{ isConnected ? "Connected" : "Disconnected" }}</p>
      </div>

      <div class="control-card">
        <button
          @click="isEnabled = !isEnabled"
          :class="{ enabled: isEnabled }"
          :disabled="!isConnected"
        >
          {{ isEnabled ? "Disable" : "Enable" }}
        </button>
      </div>

      <div class="slider-card">
        <div class="slider-container">
          <label for="slider1">Duty Cycle: {{ dutyCycle }}%</label>
          <input
            type="range"
            id="slider1"
            min="0"
            max="100"
            v-model="dutyCycle"
            :disabled="!isEnabled || !isConnected"
          />
        </div>

        <div class="slider-container">
          <label for="slider2">Frequency: {{ frequency }} Hz</label>
          <input
            type="range"
            id="slider2"
            step="25"
            min="1000"
            max="5000"
            v-model="frequency"
            :disabled="!isEnabled || !isConnected"
          />
        </div>
      </div>
    </main>
  </div>
</template>

<style>
:root {
  --primary-color: #4a6da7;
  --primary-light: #6989c3;
  --primary-dark: #345286;
  --accent-color: #f39c12;
  --text-color: #333;
  --bg-color: #f5f7fa;
  --card-bg: #ffffff;
  --disabled-color: #cccccc;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
  font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
}

body {
  background-color: var(--bg-color);
  color: var(--text-color);
}

.app-container {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  max-width: 100%;
  padding: 1rem;
}

header {
  text-align: center;
  padding: 1rem;
  margin-bottom: 1.5rem;
}

h1 {
  font-size: 1.5rem;
  color: var(--primary-dark);
}

main {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
  width: 100%;
}

.status-card,
.control-card,
.slider-card {
  background-color: var(--card-bg);
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  padding: 1.5rem;
  width: 100%;
  max-width: 500px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.status-indicator {
  width: 15px;
  height: 15px;
  border-radius: 50%;
  background-color: #ff4b4b;
  margin-right: 8px;
  display: inline-block;
}

.status-indicator.connected {
  background-color: #4caf50;
}

.status-card {
  display: flex;
  flex-direction: row;
  justify-content: center;
  align-items: center;
  font-weight: 500;
}

button {
  background-color: var(--primary-color);
  color: white;
  border: none;
  border-radius: 20px;
  padding: 10px 24px;
  font-size: 1rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: 150px;
}

button:hover {
  background-color: var(--primary-light);
}

button.enabled {
  background-color: var(--accent-color);
}

.slider-container {
  width: 100%;
  margin: 10px 0;
}

label {
  display: block;
  margin-bottom: 8px;
  font-weight: 500;
}

input[type="range"] {
  width: 100%;
  height: 10px;
  border-radius: 5px;
  background: #e0e0e0;
  outline: none;
  -webkit-appearance: none;
}

input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--primary-color);
  cursor: pointer;
}

input[type="range"]:disabled {
  background: var(--disabled-color);
}

input[type="range"]:disabled::-webkit-slider-thumb {
  background: #999;
}

/* Media queries for responsiveness */
@media (max-width: 480px) {
  .app-container {
    padding: 0.5rem;
  }

  h1 {
    font-size: 1.2rem;
  }

  .status-card,
  .control-card,
  .slider-card {
    padding: 1rem;
  }

  button {
    padding: 8px 16px;
    font-size: 0.9rem;
  }
}
</style>
