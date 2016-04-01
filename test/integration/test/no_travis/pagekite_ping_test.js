'use strict';

const chakram = require('chakram'), expect = chakram.expect;
const Config = require('config-js');

var config = new Config('./test/integration/lib/config/foxbox.js');
var Prepper = require('../../lib/testPrepper.js');

Prepper.makeSuite('Verify validity of pagekite ping endpoint', function(){
  var local_ip,tunnel_url;
  var header = new Config('./test/integration/lib/config/header.js');
  var credential = config.get('credential'); 

  it('get address from pagekite ping endpoint', function() {
    var pingUrl = config.get('pagekite.r') + '/ping';
    var pick;
    var timestamp = 0;

    return  chakram.get(pingUrl,{'json': true})
    .then(function(pingResp){
      for (var match in pingResp.body) {
        if (pingResp.body[match].tunnel_url === 
          config.get('pagekite.externalURL')) {
          // may be multiple entries.  in that case, pick latest
          if (parseInt(pingResp.body[match].timestamp) > 
            parseInt(timestamp)) {
            timestamp = pingResp.body[match].timestamp;
            pick = match;
          }
      }
    }
    local_ip = pingResp.body[pick].local_ip;
    tunnel_url = pingResp.body[pick].tunnel_url;

    expect(pingResp).to.have.status(200);
    expect(local_ip).to.match(/\d+.\d+.\d+.\d+/);
    expect(tunnel_url).equals(config.get('pagekite.externalURL'));
    expect(pingResp.body[pick].timestamp).to.match(/\d+/);
    expect(pingResp.body[pick].public_ip).to.match(/\d+.\d+.\d+.\d+/);
    });
  });

  describe('Initiate the remote connection',function(){
    it('create account via local_ip',function(){   

      local_ip = 'http://' + local_ip + ':3000';
      return chakram.post(local_ip + '/users/setup', credential)
      .then(function(setupResp) {
        expect(setupResp).to.have.status(201);
      });
    });

    it('login via the tunnel url',function(){  
      var encoded_cred = new Buffer(credential.username+
        ':'+credential.password).toString('base64');

      // supply the credential used in previous test
      header.Authorization = 'Basic ' + encoded_cred;   
      tunnel_url = 'http://' + tunnel_url + '/users/login';
      return chakram.post(tunnel_url,null,{'headers' : header})
      .then(function(loginResp) {
        expect(loginResp).to.have.status(201);
      });
    });
  });
});
