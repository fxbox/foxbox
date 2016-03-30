'use strict';

const express = require('express');
const bodyParser = require('body-parser');
const path = require('path');

var api_resp = require('./json/bridge_status.json');  // full status response

var philipsHue_server = (function() {

  // this represents the simulated lights
  var light_status = api_resp.lights; 
  var last_received_cmd = null;   
  var _server = express();
  var instance;

  function _setLight(id,field,value) { 
    light_status[id].state[field] = value;
  }

  function lastCmd() { 
    return last_received_cmd;
  }

  function lightResponse(light_id,result) {
    var response = {};
    var key = '/lights/' + light_id + '/state/on';
    response[key] = result;

    return [{'success': response}];
  }

  function turnOnLight(id) {
    _setLight(id,'on',true);
    console.log('light ' + id + ' status: ' + light_status[id].state.on);
  }

  function turnOffLight(id) {
    _setLight(id,'on',false);
    console.log('light ' + id + ' status: ' + light_status[id].state.on);
  }

  function lightStatus(id) {
    return light_status[id].state.on;
  }

  function setup(port) {

    var foxboxId;

    _server.use(function (req, res, next) { 
        // foxbox does not have the json header, so need to 
        // inject it here so the message body can be properly captured
        req.headers['content-type'] = 'application/json';
        last_received_cmd = req.method + ':' + req.originalUrl;
        next();
      });

    _server.get('/', function (req, res) {
      res.status(200).sendFile(
        path.join(__dirname,'/html/philips_initial.html'));  
    });

    _server.get('/api/:foxbox/', function (req, res) {   
         // foxbox id is created within foxbox.  Need to remember this
         if (foxboxId === undefined) {
          foxboxId = req.params.foxbox;
          res.sendStatus(200); 
        }
        else {
          res.status(200).json(api_resp);  
        }     
      });

    _server.get('/api/:foxboxId/lights', function (req, res) {
      if (req.params.foxboxId === foxboxId){ 
        res.status(200).json(light_status); 
      }
      else {
        res.status(404); 
        throw 'foxboxId mismatch: got ' + req.params.foxboxId; 
      }
    });

    _server.get('/api/:foxboxId/lights/:light_id', function (req, res) {
      if (req.params.foxboxId === foxboxId){
          // return the appropriate subset of light status
          res.status(200).json(light_status[req.params.light_id]); 
        }
        else {
          res.status(404); 
          throw 'foxboxId mismatch: got ' + req.params.foxboxId; 
        }
      });

    _server.put('/api/:foxboxId/lights/:light_id/state', 
      bodyParser.json(),function (req, res) {
        if (req.body.on) {
          turnOnLight(req.params.light_id); 
        }
        else {
          turnOffLight(req.params.light_id);      
        }
        
        var response = lightResponse(req.params.light_id,
          light_status[req.params.light_id]);
        last_received_cmd += ':' + JSON.stringify(req.body);
        res.status(200).json(response);     
      });

    instance = _server.listen(port, function () {
      console.log('Hue simulator listening on port ' + port);
    });
  }

  function stop() {
   return new Promise(resolve => {
     instance.close(function(){
       console.log('philips hue server closed');
       resolve(); // it's like if you called `callback()`
     });
   });
}

  return {setup, stop, lastCmd, lightResponse, turnOnLight, 
    turnOffLight, lightStatus};

  })();

module.exports = philipsHue_server;