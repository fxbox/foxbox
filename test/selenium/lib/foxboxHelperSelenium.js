'use strict';

const fs = require('fs');
const path = require('path');
const spawn = require('child_process').spawn;
const util = require('util');

const PROFILE_PATH = path.join(process.env.HOME, '.local/share/foxbox-tests/');
var FOXBOX_STARTUP_WAIT_TIME_IN_MS = 5000;
var foxboxInstance;

var helper = (function() {

  const FOXBOX_PORT = 3331;
  const HOST_URL = util.format('http://localhost:%d/', FOXBOX_PORT);

  function _removeFileIfItExists(filePath, errMsg) {
    try {
      fs.unlinkSync(filePath);
    } catch (e) {
      if (e.code === 'ENOENT') {
        console.log(errMsg);
      }
    }
  }

  function removeUsersDB() {
    _removeFileIfItExists(PROFILE_PATH + 'users_db.sqlite',
      'User DB not found!');
  }

  function fullOptionStart(callback) {
    foxboxInstance = spawn('./target/debug/foxbox', [
      '--disable-tls',
      '--port', FOXBOX_PORT,
      '--profile', PROFILE_PATH,
    ], {stdio: 'inherit'});
    setTimeout(callback, FOXBOX_STARTUP_WAIT_TIME_IN_MS);
  }

  function killFoxBox() {
    foxboxInstance.kill('SIGINT');
  }



  return {
    removeUsersDB, fullOptionStart, killFoxBox, HOST_URL
  };
})();

module.exports = helper;
