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


  return {removeUsersDB, fullOptionStart, killFoxBox};
})();

module.exports = helper;
