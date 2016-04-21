'use strict';

const chakram = require('chakram'), expect = chakram.expect;
const Config = require('config-js');
const isoDate = require('iso-date');

var config = new Config('./test/integration/lib/config/foxbox.js');
var Prepper = require('../lib/testPrepper.js');

Prepper.makeSuite('Control camera locally',function(){

  var cameraService;

  before(Prepper.foxboxManager.foxboxLogin);

  it('check simulated camera is registered',function(){        
    return chakram.get(Prepper.foxboxManager.serviceListURL)
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
    return chakram.put(Prepper.foxboxManager.setterURL, payload)
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
    return chakram.put(Prepper.foxboxManager.getterURL, payload)
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
    return chakram.put(Prepper.foxboxManager.getterURL, payload)
    .then(function(cmdResponse) {
      expect(cmdResponse).to.have.status(200);
      expect(cmdResponse).to.have.header('content-type', 'image/jpeg');
      expect(cmdResponse).to.have.header('content-length', '212502');
    });
  });    

});
