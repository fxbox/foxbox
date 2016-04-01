'use strict';

const chakram = require('chakram'), expect = chakram.expect;
const Config = require('config-js');

var config = new Config('./test/integration/lib/config/foxbox.js');
var Prepper = require('../lib/testPrepper.js');

Prepper.makeSuite('Control lights locally',function(){
  var lightinfo;
  var header = new Config('./test/integration/lib/config/header.js');
  var credential = config.get('credential'); 

  var setupURL = config.get('foxbox.url') + '/users/setup';
  var loginURL = config.get('foxbox.url') + '/users/login';
  var serviceURL = config.get('foxbox.url') + '/services/';
  var serviceListURL = serviceURL + 'list';  
  
  it('create a new credential',function(){

    return chakram.post(setupURL,credential)
    .then(function(setupResp){
      expect(setupResp).to.have.status(201);
    });
  });

  it('login to foxbox',function(){
    var encoded_cred = new Buffer(credential.username+
      ':'+credential.password).toString('base64');

    // supply the credential used in previous test
    header.Authorization = 'Basic ' + encoded_cred;

    return  chakram.post(loginURL,null,{'headers' : header})
    .then(function(loginResp){
      expect(loginResp).to.have.status(201);
    });
  });

  describe ('once logged in', function () {
    before(function() {
      return chakram.post(loginURL,null,{'headers' : header})
      .then(function(resp){
       header.Authorization = 'Bearer ' + resp.body.session_token;
       chakram.setRequestDefaults({'headers': header});
     });
    });

    it('check 3 bulbs are registered',function(){

      return chakram.get(serviceListURL)
      .then(function(listResponse) {
        expect(listResponse.body.length).equals(3);
        expect(listResponse).to.have.status(200);
      });
    });

    describe('once the lights are known', function(){
      before(function() {
        // turn off all lights in the simulator
        Prepper.philipshue_server.turnOffLight(1);
        Prepper.philipshue_server.turnOffLight(2);
        Prepper.philipshue_server.turnOffLight(3);

        return chakram.get(serviceListURL)
        .then(function(listResponse) {
          lightinfo = listResponse.body;
        });
      });

      // Currently, there is no mapping between the foxbox
      // issues id and the philips hue id until the tag feature is implemented
      it('Turn on all lights', function(){
        return Prepper.turnOnLight(lightinfo, serviceURL, 0)
        .then(function(cmdResponse) {
          Prepper.expectLightIsOn(cmdResponse);
          return Prepper.turnOnLight(lightinfo, serviceURL, 1);
        })
        .then(function(cmdResponse) {
          Prepper.expectLightIsOn(cmdResponse);
          return Prepper.turnOnLight(lightinfo, serviceURL, 2);
        })
        .then(function(cmdResponse) {
         Prepper.expectLightIsOn(cmdResponse);
         
         // check no lights are turned off now
         expect(Prepper.philipshue_server.lightStatus(1) &&
          Prepper.philipshue_server.lightStatus(2) &&
          Prepper.philipshue_server.lightStatus(3)).equals(true);
       });
      });
});
});
});