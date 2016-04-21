'use strict';
const Config = require('config-js');

var config = new Config('./test/integration/lib/config/foxbox.js');
var philipshue_server = require('../lib/philipsHue_server.js');
var ipcamera_server = require('../lib/ipcamera_server.js');
var webPush_server = require('../lib/webpush_server.js');
var nupnp_server = require('../lib/nupnp_PhilipsHue.js');
var foxboxManager = require('../lib/foxboxHelper.js');
var config = new Config('./test/integration/lib/config/foxbox.js');

var testPrepper = (function() {

  function beforeTest(done) { 

    console.log('test started');
    nupnp_server.start(config.get('nupnp_server.id'),
      config.get('philips_hue.url') +':'+config.get('philips_hue.port'),
      config.get('nupnp_server.port'));
    
    //Start the foxbox and associated simulators
    philipshue_server.setup(config.get('philips_hue.port'));
    ipcamera_server.setup();
    webPush_server.setup();
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
        var promises = [nupnp_server.stop(), 
        philipshue_server.stop(),ipcamera_server.stop(),webPush_server.stop()];
        return Promise.all(promises); 
      });
    });
  }

  return { makeSuite, 
    philipshue_server,ipcamera_server, webPush_server, foxboxManager};
})();

module.exports = testPrepper;