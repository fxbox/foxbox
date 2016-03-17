'use strict';
const Config = require('config-js');
const chakram = require('chakram'), expect = chakram.expect;

var config = new Config('./test/integration/lib/config/foxbox.js');
var philipshue_server = require('../lib/philipsHue_server.js');
var nupnp_server = require('../lib/nupnp_PhilipsHue.js');
var foxboxManager = require('../lib/foxboxHelper.js');
var config = new Config('./test/integration/lib/config/foxbox.js');

var testPrepper = (function() {

  function expectLightIsOn(response) {
   expect(response).to.have.status(200);
   expect(response.body.result).equals('success');
   expect(philipshue_server.lastCmd()).to.contain('"on":true');
  }

  function turnOnLight(lightinfo, serviceURL, index) {
    return chakram.put(serviceURL + lightinfo[index].id + 
      '/state', {'on': true});
  }

  function beforeTest(done) { 

    console.log('test started');
    nupnp_server.start(config.get('nupnp_server.id'),
      config.get('philips_hue.url') +':'+config.get('philips_hue.port'),
      config.get('nupnp_server.port'));
    
    philipshue_server.setup(config.get('philips_hue.port'));

    foxboxManager.fullOptionStart(done);        
  }

  function makeSuite(desc, test) {
    describe(desc, function () {
      this.timeout(5000000);
      before(beforeTest);
      test();
      after(function() {
        foxboxManager.killFoxBox();
        foxboxManager.removeUsersDB();
        var promises = [nupnp_server.stop(), philipshue_server.stop()];
        return Promise.all(promises); 
      });
    });
  }

  return {expectLightIsOn, turnOnLight, makeSuite, philipshue_server};
})();

module.exports = testPrepper;