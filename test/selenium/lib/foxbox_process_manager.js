'use strict';

const find = require('find');
const fs = require('fs');
const path = require('path');
const spawn = require('child_process').spawn;
const format = require('util').format;

const PROFILE_PATH = path.join(process.env.HOME, '.local/share/foxbox-tests/');
const STARTUP_WAIT_TIME_IN_MS = 5000;

function FoxboxManager() {
  this._foxboxInstance = null;
}

FoxboxManager.PORT = 3331;
FoxboxManager.HOST_URL = format('http://localhost:%d/', FoxboxManager.PORT);

FoxboxManager.prototype = {
  start: function() {
    this._foxboxInstance = spawn('./target/debug/foxbox', [
      '--disable-tls',
      '--port', FoxboxManager.PORT,
      '--profile', PROFILE_PATH,
    ], { stdio: 'inherit' });

    return new Promise(resolve => {
      setTimeout(resolve, STARTUP_WAIT_TIME_IN_MS);
    });
  },

  kill: function() {
    this._foxboxInstance.kill('SIGINT');
  },

  cleanData: function() {
    return this._deleteAllProfileButCertificates();
  },

  /* Keeping certificates allows foxbox to have the same public domain name.
   * As a consequence, tests don't fail because many foxboxes are detected on
   * github.io.
   */
  _deleteAllProfileButCertificates: function() {
    return new Promise(resolve => {
      find.file(/^((?!\.pem).)*$/, PROFILE_PATH, files => {
        var promises = files.map(file => new Promise(res => {
          fs.unlink(file, res);
        }));

        Promise.all(promises).then(resolve);
      });
    });
  }
};

module.exports = FoxboxManager;
