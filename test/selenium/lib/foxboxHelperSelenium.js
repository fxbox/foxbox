'use strict';
const fs = require('fs');
const path = require('path');

const spawn = require('child_process').spawn;

var FOXBOX_STARTUP_WAIT_TIME_IN_MS = 5000;
var foxboxInstance;

var helper = (function() {

  function _removeFileIfItExists(filename,errMsg) {
    try {
      fs.unlinkSync(path.join(process.env.HOME,
      filename));
    } catch (e) {
      if (e.code === 'ENOENT') {
        console.log(errMsg);
      }
    }
  }

  function removeUsersDB() {
    _removeFileIfItExists('/.local/share/foxbox-debug/users_db.sqlite',
      'User DB not found!');
  }

  function fullOptionStart(callback) {
  foxboxInstance = spawn('./target/debug/foxbox',
    ['-c',
    '--disable-tls']); 
  setTimeout(callback, FOXBOX_STARTUP_WAIT_TIME_IN_MS);
  }

  function killFoxBox() {
    foxboxInstance.kill('SIGINT');
  }

  return {
    removeUsersDB, fullOptionStart, killFoxBox,
    };
})();

module.exports = helper;
