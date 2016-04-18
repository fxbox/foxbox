'use strict';

const chakram = require('chakram'), expect = chakram.expect;
const Config = require('config-js');

var config = new Config('./test/integration/lib/config/foxbox.js');
var Prepper = require('../lib/testPrepper.js');

Prepper.makeSuite('Control lights locally',function(){
  var header = new Config('./test/integration/lib/config/header.js');
  var credential = config.get('credential');
  var setupURL = config.get('foxbox.url') + '/users/setup';
  var loginURL = config.get('foxbox.url') + '/users/login';
  var channelURL = config.get('foxbox.url') + '/api/v1/channels';
  var getURL = channelURL + '/get';
  var setURL = channelURL + '/set';
  var getterPayload = [{'kind': 'LightOn'}];
  var lights;
  
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
      
      // collect all getters for the lightbulbs
      return chakram.put(getURL,getterPayload)
      .then(function(listResponse) {
        lights = Object.keys(listResponse.body);
        expect(lights.length).equals(3);
        expect(listResponse).to.have.status(200);
      });
    });

    describe('manipulate lights', function () {
      before(function() {
        // turn off all lights in the simulator
        Prepper.philipshue_server.turnOffLight(1);
        Prepper.philipshue_server.turnOffLight(2);
        Prepper.philipshue_server.turnOffLight(3);

        return chakram.put(getURL,getterPayload)
        .then(function(listResponse) {
          lights = Object.keys(listResponse.body);
          expect(lights.length).equals(3);
          expect(listResponse).to.have.status(200);
        });
      });

      // Currently, there is no mapping between the foxbox
      // id and the philips hue id until the tag feature is implemented
      it('Turn on all lights one by one', function(){
        var lightID = lights[0].replace('getter','setter');
        var payload = {'select': {'id': lightID}, 'value': { 'OnOff': 'On' } };
        
        return chakram.put(setURL,payload)
        .then(function(cmdResponse) {
         expect(cmdResponse).to.have.status(200);
         expect(cmdResponse.body[lightID]).equals(null);
         lightID = lights[1].replace('getter','setter');
         payload = {'select': {'id': lightID}, 'value': { 'OnOff': 'On' } };
         return chakram.put(setURL,payload);
       })
        .then(function(cmdResponse) {
          expect(cmdResponse).to.have.status(200);
          expect(cmdResponse.body[lightID]).equals(null);
          lightID = lights[2].replace('getter','setter');
          payload = {'select': {'id': lightID}, 'value': { 'OnOff': 'On' } };
          return chakram.put(setURL,payload);
        })
        .then(function(cmdResponse) {
          expect(cmdResponse).to.have.status(200);
          expect(cmdResponse.body[lightID]).equals(null);
          expect(Prepper.philipshue_server.areAllLightsOn()).to.be.true;
          
          //check all lights are reported to be on
          return chakram.put(getURL,getterPayload);
        })
        .then(function(listResponse) {
          expect(listResponse).to.have.status(200);
          lights = Object.keys(listResponse.body);
          expect(lights.length).equals(3);

          lights.forEach(light => {
            expect(listResponse.body[light].OnOff).equals('On'); 
          });
        }); 
      });

      it('Turn off all lights at once', function() {
        
        var payload = {'select': {'kind': 'LightOn'}, 'value': {'OnOff':'Off'}};

        return chakram.put(setURL,payload)
        .then(function(cmdResponse) {
          expect(cmdResponse).to.have.status(200);
          expect(Prepper.philipshue_server.areAllLightsOn()).to.be.false;
          
          //check all lights are reported to be off
          return chakram.put(getURL,getterPayload);
        })
        .then(function(listResponse) {
          lights = Object.keys(listResponse.body);
          expect(lights.length).equals(3);
          expect(listResponse).to.have.status(200);

          lights.forEach(light => {
            expect(listResponse.body[light].OnOff).equals('Off'); 
          });
        }); 
      });
    });
  });
});
