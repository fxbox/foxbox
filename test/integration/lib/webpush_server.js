'use strict';

const express = require('express');
const Config = require('config-js');
const getRawBody = require('raw-body');
const assert = require('assert');
const crypto = require('crypto');
const ece = require('http_ece');
const typer = require('media-typer');
var config = new Config('./test/integration/lib/config/foxbox.js');

var webpush_server = (function() {
  var serverECDH = crypto.createECDH('prime256v1');
  var serverPublicKey = serverECDH.generateKeys('base64');
  var _server = express();
  
  var port = config.get('webpush.port');
  var endpoint = config.get('webpush.endpoint');
  
  var incomingPushMsg;
  var serverInstance;

  function getEndpointURI() {
    return config.get('webpush.ip') + ':' + port + endpoint;
  }

  function getPublicKey() {
    return serverPublicKey;
  }

  function getDecodedPushMsg() {
    return incomingPushMsg;
  }

  function setup() {

    //capture the payload of the message in a buffer
    _server.use(function (req, res, next) {      
      getRawBody(req, {
        length: req.headers['content-length'],
        limit: '1mb',
        encoding: typer.parse('application/octet-stream').parameters.charset
      }, function (err, data) {
          if (err) {return next(err);}
          req.data = data;
          next();
        });
    });
  
    // Handles the incoming ECDH encrypted webpush message from foxbox
    _server.post(endpoint, function (req, res) {
      assert.equal(req.headers['content-encoding'], 
        'aesgcm128', 'Content-Encoding header correct');
      // Collect salt, publickey, and the message
      var salt = req.headers.encryption.match('salt=(.*);')[1];
      var pubkey = req.headers['encryption-key']
      .match('keyid=p256dh;dh=(.*)')[1];
      var encryptedBuffer = new Buffer(req.data,'binary');
      
      // compute shared secret using the provided public key of foxbox
      var sharedSecret = serverECDH.computeSecret(pubkey,'base64');
      ece.saveKey('webpushKey', sharedSecret);
        
      // Decrypt the message using the shared secret and provided salt
      var decrypted = ece.decrypt(encryptedBuffer, {
              keyid: 'webpushKey',
              salt: salt,
              padSize: 1,
            });
      incomingPushMsg = decrypted.toString();        
      console.log('message: ' + incomingPushMsg);
      res.sendStatus(200);     
    });

    serverInstance = _server.listen(port, function () {
      console.log('Webpush server listening on port ' + port);
    });
  }

  function stop() {
   return new Promise(resolve => {
     serverInstance.close(function(){
       console.log('Webpush server closed');
       resolve();
     });
   });
  }

  return {setup, stop, getEndpointURI, getPublicKey, getDecodedPushMsg};

  })();

module.exports = webpush_server;