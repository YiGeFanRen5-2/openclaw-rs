// N-API binding loader for node-bridge
const path = require('path');
const binding = require(path.join(__dirname, 'index.node'));
module.exports = binding;
