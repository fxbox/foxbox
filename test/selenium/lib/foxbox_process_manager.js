'use strict';

const find = require('find');
const fs = require('fs');
const path = require('path');
const spawn = require('child_process').spawn;
const util = require('util');

const PROFILE_PATH = path.join(process.env.HOME, '.local/share/foxbox-tests/');
var FOXBOX_STARTUP_WAIT_TIME_IN_MS = 5000;
var foxboxInstance;

var foxboxProcessManager = (function() {

  const FOXBOX_PORT = 3331;
  const HOST_URL = util.format('http://localhost:%d/', FOXBOX_PORT);

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

  /* Keeping certificates allows foxbox to have the same public domain name.
   * As a consequence, tests don't fail because many foxboxes are detected on
   * github.io.
   */
  function _deleteAllProfileButCertificates() {
    return new Promise(resolve => {
      find.file(/^((?!\.pem).)*$/, PROFILE_PATH, files => {
        var promises = files.map(file => new Promise(res => {
          fs.unlink(file, res);
        }));

        Promise.all(promises).then(resolve);
      });
    });
  }

  function cleanData() {
    return _deleteAllProfileButCertificates();
  }

  return {
    fullOptionStart, killFoxBox, cleanData, HOST_URL
  };
})();

module.exports = foxboxProcessManager;
