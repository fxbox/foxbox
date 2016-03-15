'use strict';

const spawn = require('child_process').spawn;
const chakram = require('chakram'), expect = chakram.expect;
const Config = require('config-js');

var philipshue_server = require('../lib/philipsHue_server.js');
var nupnp_server = require('../lib/nupnp_PhilipsHue.js');
var config = new Config('./test/integration/lib/config/foxbox.js');
var header = new Config('./test/integration/lib/config/header.js');

describe('Initiate the connection with foxbox as Philips Hue hub',function(){
  var credential = config.get('credential'); 
  var FOXBOX_STARTUP_WAIT_TIME_IN_MS = 3000;
  var lightinfo; // to store the light status received from foxbox
  var foxbox_process;
  
  before(function(done){
    this.timeout(500000);

    const hue_location = 'localhost:' + config.get('philips_hue.port');
    nupnp_server.start(config.get('nupnp_server.id'),
      hue_location,config.get('nupnp_server.port'));
    
    philipshue_server.setup(config.get('philips_hue.port'));

    philipshue_server.turnOffLight(1);
    philipshue_server.turnOffLight(2);
    philipshue_server.turnOffLight(3);

    foxbox_process = spawn('./target/debug/foxbox', 
      ['-c', 'philips_hue;nupnp_url;http://localhost:'+ 
      config.get('nupnp_server.port')+'/']);

    // give time until foxbox is operational
    setTimeout(done, FOXBOX_STARTUP_WAIT_TIME_IN_MS);  
  });
  
  after(function(){
    foxbox_process.kill();
  });
  
  it('create a new credential',function(){

    return chakram.post(config.get('foxbox.url') + '/users/setup',credential)
    .then(function(setupResp){
      expect(setupResp).to.have.status(201);
    });
  });

  it('login to foxbox',function(){
    var key = credential.username+':'+credential.password;
    var encoded_cred = new Buffer(key).toString('base64');

    // supply the credential used in previous test
    header.Authorization = 'Basic ' + encoded_cred;  
    
    return  chakram.post(config.get('foxbox.url') + 
      '/users/login',null,{'headers' : header})
    .then(function(loginResp){
      expect(loginResp).to.have.status(201);
    });
  });

  describe ('once logged in', function () {
    before(function() {
      return chakram.post(config.get('foxbox.url') + 
        '/users/login',null,{'headers' : header})
      .then(function(resp){
       header.Authorization = 'Bearer ' + resp.body.session_token;
       chakram.setRequestDefaults({'headers': header});
     });
    });

    it('check 3 bulbs are registered',function(){   

      return chakram.get(config.get('foxbox.url') + '/services/list')
      .then(function(listResponse) {
        expect(listResponse.body.length).equals(3);
        expect(listResponse).to.have.status(200);
      });
    });

    describe('once the lights are known', function(){
      before(function() {
        return chakram.get(config.get('foxbox.url') + '/services/list')
        .then(function(listResponse) {
          lightinfo = listResponse.body;
        });
      });

      // Currently, there is no mapping between the foxbox 
      // issues id and the philips hue id until the tag feature is implemented
      it('Turn on all lights', function(){
        
        return chakram.put(config.get('foxbox.url') + '/services/' + 
          lightinfo[0].id + '/state', {'on': true})
        .then(function(cmdResponse) {
         expect(cmdResponse).to.have.status(200);
         expect(cmdResponse.body.result).equals('success');
         expect(philipshue_server.lastCmd()).to.contain('"on":true');
         return chakram.put(config.get('foxbox.url') + '/services/' + 
          lightinfo[1].id + '/state', {'on': true});})
        .then(function(cmdResponse) {
         expect(cmdResponse).to.have.status(200);
         expect(cmdResponse.body.result).equals('success');
         expect(philipshue_server.lastCmd()).to.contain('"on":true');
         return chakram.put(config.get('foxbox.url') + '/services/' + 
        lightinfo[2].id + '/state', {'on': true});})
        .then(function(cmdResponse) {
         expect(cmdResponse).to.have.status(200);
         expect(cmdResponse.body.result).equals( 'success');
         expect(philipshue_server.lastCmd()).to.contain('"on":true');
         
         // check no lights are turned off now
         expect(philipshue_server.lightStatus(1) && 
          philipshue_server.lightStatus(2) && 
          philipshue_server.lightStatus(3)).equals(true);
       });
      });
    });
  });
});
