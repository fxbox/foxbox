'use strict';

const chakram = require('chakram'), expect = chakram.expect;
const Config = require('config-js');
const isoDate = require('iso-date');

var config = new Config('./test/integration/lib/config/foxbox.js');
var Prepper = require('../lib/testPrepper.js');

Prepper.makeSuite('Control camera locally',function(){
  var header = new Config('./test/integration/lib/config/header.js');
  var credential = config.get('credential'); 

  var setupURL = config.get('foxbox.url') + '/users/setup';
  var loginURL = config.get('foxbox.url') + '/users/login';
  var serviceURL = config.get('foxbox.url') + '/api/v1';
  var serviceListURL = serviceURL + '/services'; 
  var getterURL = serviceURL + '/channels/get';
  var setterURL = serviceURL + '/channels/set';
  var cameraService;

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

      it('check simulated camera is registered',function(){        
        return chakram.get(serviceListURL)
        .then(function(listResponse) {
          for (var entry of listResponse.body) {
            if (entry.adapter === 'ip-camera@link.mozilla.org') {
                cameraService = entry;                
             }
          }
          expect(listResponse).to.have.status(200);
          expect(cameraService.properties.udn).
            equals(config.get('ipCamera.udn'));
          
        });
      });
      
      it('take a picture',function(){
        var setter = 'setter:' + 
        cameraService.id.replace('service:','snapshot.');
        var payload = {'select': {'id': setter}, 'value': {'Unit': {}}};
        return chakram.put(setterURL, payload)
        .then(function(cmdResponse) {
          expect(cmdResponse).to.have.status(200);
          expect(cmdResponse.body[setter]).equals(null);
        });
      }); 

      // Image filesnames have YYYY-MM-DD prefix. make sure the last image
      // taken is from today
      it('get a list of images taken',function(){        
        var datePrefix = isoDate(new Date());

        var getter = 'getter:'+ 
        cameraService.id.replace('service:','image_list.');
        var payload = {'id': getter};
        return chakram.put(getterURL, payload)
        .then(function(cmdResponse) {
          var imageList = cmdResponse.body[getter].Json;
          var res = imageList[imageList.length - 1].match(datePrefix);
          expect(cmdResponse).to.have.status(200);
          expect(imageList.length).above(0);
          expect(res.index).equal(0);
        });
      }); 
      
      // Checks the correct size of bytes is received
      it('download the picture',function(){
        var getter = 'getter:' + 
        cameraService.id.replace('service:','image_newest.');
        var payload = {'id': getter};
        return chakram.put(getterURL, payload)
        .then(function(cmdResponse) {
          expect(cmdResponse).to.have.status(200);
          expect(cmdResponse).to.have.header('content-type', 'image/jpeg');
          expect(cmdResponse).to.have.header('content-length', '212502');
        });
      }); 
     
  });



});
