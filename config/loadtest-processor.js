// Artillery.js processor for load testing scenarios

const crypto = require('crypto');

module.exports = {
  // Generate random strings for test data
  randomString: (context, events, done) => {
    const length = Math.floor(Math.random() * 100) + 20; // 20-120 chars
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 ';
    let result = '';
    
    for (let i = 0; i < length; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    
    context.vars.randomString = result;
    return done();
  },

  // Generate realistic MCP requests
  mcpRequest: (context, events, done) => {
    const prompts = [
      "Analyze the sensor data for anomalies",
      "Summarize the IoT device status",
      "Generate recommendations based on telemetry",
      "Process the edge computing workload",
      "Optimize the resource allocation",
      "Predict maintenance requirements",
      "Calculate energy efficiency metrics",
      "Monitor system performance indicators"
    ];
    
    const tools = [
      {
        type: "function",
        function: {
          name: "analyze_data",
          description: "Analyze sensor data for patterns"
        }
      },
      {
        type: "function", 
        function: {
          name: "generate_report",
          description: "Generate status report"
        }
      }
    ];

    context.vars.mcpPrompt = prompts[Math.floor(Math.random() * prompts.length)];
    context.vars.mcpTools = Math.random() > 0.5 ? tools : [];
    
    return done();
  },

  // Simulate edge device characteristics
  edgeDevice: (context, events, done) => {
    const devices = [
      { type: "raspberry-pi", memory: "1GB", cpu: "ARM64" },
      { type: "jetson-nano", memory: "4GB", cpu: "ARM64" },
      { type: "esp32", memory: "512KB", cpu: "ESP32" },
      { type: "x86-edge", memory: "8GB", cpu: "x64" }
    ];
    
    const device = devices[Math.floor(Math.random() * devices.length)];
    
    context.vars.deviceType = device.type;
    context.vars.deviceMemory = device.memory;
    context.vars.deviceCpu = device.cpu;
    
    return done();
  },

  // Custom metrics collection
  collectMetrics: (context, events, done) => {
    events.emit('counter', 'custom.requests.total', 1);
    
    if (context.vars.latency) {
      events.emit('histogram', 'custom.latency.ms', context.vars.latency);
    }
    
    if (context.vars.deviceType) {
      events.emit('counter', `custom.device.${context.vars.deviceType}`, 1);
    }
    
    return done();
  },

  // Error handling
  handleError: (context, events, done) => {
    if (context.vars.statusCode >= 400) {
      events.emit('counter', `custom.errors.${context.vars.statusCode}`, 1);
    }
    
    return done();
  }
};