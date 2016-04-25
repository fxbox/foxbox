'use strict';

const express = require('express');
const Config = require('config-js');
const getRawBody = require('raw-body');
const assert = require('assert');
const crypto = require('crypto');
const ece = require('http_ece');
const typer = require('media-typer');
var urlBase64 = require('urlsafe-base64');
var config = new Config('./test/integration/lib/config/foxbox.js');

var webpush_server = (function() {
  var serverECDH = crypto.createECDH('prime256v1');
  var serverPublicKey = serverECDH.generateKeys('base64');
  var userAuth = urlBase64.encode(crypto.randomBytes(16));
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

  function getUserAuth() {
    return userAuth;
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
  
    // Handles the incoming ECDH encrypted webpush message 
    // (with the new webpush standard) from foxbox
    _server.post(endpoint, function (req, res) {
      var encoding = req.headers['content-encoding'];
      var keyfield, decrypted;

      if (encoding == 'aesgcm128') {
        keyfield = 'encryption-key';
      }
      else if (encoding == 'aesgcm') {
        keyfield = 'crypto-key';
      }
      else { // unknown encoding
        assert(false, 'unknown encoding');
      }
      
      // Collect salt, publickey, and the message
      var salt = req.headers.encryption.match('salt=(.*);')[1];
      var pubkey = req.headers[keyfield]
      .match('keyid=p256dh;dh=(.*)')[1];
      var encryptedBuffer = new Buffer(req.data,'binary');
      
      // Decrypt the message using the shared secret and provided salt
      if (encoding == 'aesgcm') {
        ece.saveKey('receiver', serverECDH, 'P-256');  
        decrypted = ece.decrypt(encryptedBuffer, {
              keyid: 'receiver',
              dh: pubkey,
              salt: salt,
              authSecret: userAuth
            });
      }
      else if (encoding == 'aesgcm128') {
        var sharedSecret = serverECDH.computeSecret(pubkey,'base64');
        ece.saveKey('webpushKey', sharedSecret);
        decrypted = ece.decrypt(encryptedBuffer, {
          keyid: 'webpushKey',
          salt: salt,
          padSize: 1,
        });
      }
      
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

  return {setup, stop, getEndpointURI, getPublicKey, 
    getUserAuth, getDecodedPushMsg};

  })();

module.exports = webpush_server;