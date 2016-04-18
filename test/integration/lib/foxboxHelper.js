'use strict';
var fs = require('fs');
var path = require('path');
const Config = require('config-js');

const spawn = require('child_process').spawn;

var config = new Config('./test/integration/lib/config/foxbox.js');
var FOXBOX_STARTUP_WAIT_TIME_IN_MS = 5000;
var foxboxInstance;

var helper = (function() {

  function removeUsersDB() {
    var filePath = path.join(process.env.HOME,
      '/.local/share/foxbox/users_db.sqlite');
    fs.unlinkSync(filePath);
  }

  function fullOptionStart(callback) {
  foxboxInstance = spawn('./target/debug/foxbox',
    ['-c',  config.get('nupnp_server.param')+';'+
    config.get('nupnp_server.url')+':'+
    config.get('nupnp_server.port')+'/',
    '--disable-tls'], {stdio: 'inherit'} ); // TODO TLS not yet supported
  setTimeout(callback, FOXBOX_STARTUP_WAIT_TIME_IN_MS);
  }

  function killFoxBox() {
    foxboxInstance.kill('SIGINT');
  }

  function getLatestIPFromPingSrv(body) {
    var pick;
    var timestamp = 0;

    for (var match in body) {
      // may be multiple entries.  in that case, pick latest
      if (parseInt(body[match].timestamp) > 
        parseInt(timestamp)) {
        timestamp = body[match].timestamp;
        pick = match;
      }
    }
    return pick;
  }

  return {removeUsersDB, fullOptionStart, killFoxBox, getLatestIPFromPingSrv};
})();

module.exports = helper;
