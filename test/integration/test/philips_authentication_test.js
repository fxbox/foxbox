'use strict';

const chakram = require('chakram'), expect = chakram.expect;

var Prepper = require('../lib/make_suite.js');

Prepper.makeSuite('Test Hue Authentication', function () {

  var getterPayload = [{ 'feature': 'light/is-on' }];
  var lights;

  before(Prepper.turnOnAllSimulators);
  before(Prepper.turnOnFoxbox);

  it('Send light query without authentication', function () {

    // collect all getters for the lightbulbs
    return chakram.put(Prepper.foxboxManager.getterURL, getterPayload)
      .then(function (listResponse) {
        console.log("no auth: " + JSON.stringify(listResponse));
        lights = Object.keys(listResponse.body);
        expect(lights.length).equals(0);
        expect(listResponse).to.have.status(200);
      });
  });

  describe('Authenticate with Philips Hue', function () {
    before(Prepper.foxboxManager.foxboxLogin);
    before(Prepper.philipshue_server.pressButton);

    it('Send light query after authentication', function () {

      // collect all getters for the lightbulbs
      return chakram.put(Prepper.foxboxManager.getterURL, getterPayload)
        .then(function (listResponse) {
          console.log("with auth: " + JSON.stringify(listResponse));
          lights = Object.keys(listResponse.body);
          expect(lights.length).equals(3);
          expect(listResponse).to.have.status(200);
        });
    });
  });
});
