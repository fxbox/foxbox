'use strict';

const chakram = require('chakram'), expect = chakram.expect;

var Prepper = require('../lib/testPrepper.js');

Prepper.makeSuite('Control lights locally',function(){

  var getterPayload = [{'kind': 'LightOn'}];
  var lights;
  
  before(Prepper.foxboxManager.foxboxLogin);

  it('check 3 bulbs are registered',function(){
    
    // collect all getters for the lightbulbs
    return chakram.put(Prepper.foxboxManager.getterURL,getterPayload)
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

      return chakram.put(Prepper.foxboxManager.getterURL,getterPayload)
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
      
      return chakram.put(Prepper.foxboxManager.setterURL,payload)
      .then(function(cmdResponse) {
       expect(cmdResponse).to.have.status(200);
       expect(cmdResponse.body[lightID]).equals(null);
       lightID = lights[1].replace('getter','setter');
       payload = {'select': {'id': lightID}, 'value': { 'OnOff': 'On' } };
       return chakram.put(Prepper.foxboxManager.setterURL,payload);
     })
      .then(function(cmdResponse) {
        expect(cmdResponse).to.have.status(200);
        expect(cmdResponse.body[lightID]).equals(null);
        lightID = lights[2].replace('getter','setter');
        payload = {'select': {'id': lightID}, 'value': { 'OnOff': 'On' } };
        return chakram.put(Prepper.foxboxManager.setterURL,payload);
      })
      .then(function(cmdResponse) {
        expect(cmdResponse).to.have.status(200);
        expect(cmdResponse.body[lightID]).equals(null);
        expect(Prepper.philipshue_server.areAllLightsOn()).to.be.true;
        
        //check all lights are reported to be on
        return chakram.put(Prepper.foxboxManager.getterURL,getterPayload);
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

      return chakram.put(Prepper.foxboxManager.setterURL,payload)
      .then(function(cmdResponse) {
        expect(cmdResponse).to.have.status(200);
        expect(Prepper.philipshue_server.areAllLightsOn()).to.be.false;
        
        //check all lights are reported to be off
        return chakram.put(Prepper.foxboxManager.getterURL,getterPayload);
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
