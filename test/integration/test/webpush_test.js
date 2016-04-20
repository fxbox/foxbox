'use strict';

const chakram = require('chakram'), expect = chakram.expect;

var Prepper = require('../lib/testPrepper.js');

Prepper.makeSuite('Test Push Service locally',function(){

  before(Prepper.foxboxManager.foxboxLogin);

  it('Check webpush service is registered',function(){  
    var pushService = false;

    return chakram.get(Prepper.foxboxManager.serviceListURL)
    .then(function(listResponse) {
      expect(listResponse).to.have.status(200);
      for (var entry of listResponse.body) {
        if (entry.adapter === 'webpush@link.mozilla.org') {
          pushService = true;                
        }
      }
      expect(pushService).equals(true);
    });
  });

  it('Add push subscription',function(){     
    var pushURI = Prepper.webPush_server.getEndpointURI();
    var pushkey = Prepper.webPush_server.getPublicKey();
    var setter = 'setter:subscribe.webpush@link.mozilla.org';
    var getter =  'getter:subscription.webpush@link.mozilla.org';
    var setterPayload = {
      'select': {
        'id':setter
      }, 
      'value': {
        'Json': {
          'subscriptions':[{
            'push_uri':pushURI,
            'public_key':pushkey
          }]}}};
    var getterPayload = {'id': getter};    

    return chakram.put(Prepper.foxboxManager.setterURL,setterPayload)
    .then(function(cmdResp){
     expect(cmdResp).to.have.status(200);
     // When there is no error, the payload returns null with the setter
     expect(cmdResp.body[setter]).equals(null);
     return chakram.put(Prepper.foxboxManager.getterURL,getterPayload);
    })
    .then(function(cmdResp){
      expect(cmdResp).to.have.status(200);
      expect(cmdResp.body[getter].Json.subscriptions[0].public_key)
      .equals(Prepper.webPush_server.getPublicKey());
      expect(cmdResp.body[getter].Json.subscriptions[0].push_uri)
      .equals(Prepper.webPush_server.getEndpointURI());
    });
  });

  it('Set resources to receive notification',function(){     
    var getter =  'getter:resource.webpush@link.mozilla.org';   
    var setter = 'setter:resource.webpush@link.mozilla.org';
    var setterPayload = {
      'select': {
        'id': setter}, 
        'value': {
          'Json': {
            'resources':['livingroom', 'washroom']}}};
    var getterPayload = {'id': getter};

    return chakram.put(Prepper.foxboxManager.setterURL,setterPayload)
    .then(function(cmdResp){
     expect(cmdResp).to.have.status(200);
     expect(cmdResp.body[setter]).equals(null);
     return chakram.put(Prepper.foxboxManager.getterURL,getterPayload);
    })
    .then(function(cmdResp){
      expect(cmdResp).to.have.status(200);
      expect(cmdResp.body[getter]
        .Json.resources[0]).equals('livingroom');
      expect(cmdResp.body[getter]
        .Json.resources[1]).equals('washroom');
    });  
  });

  it('Accepts notification trigger requests',function(){
    var setter = 'setter:notify.webpush@link.mozilla.org';
    var resource = 'washroom';
    var notificationText = 'lights on!';
    var payload = {
      'select': {
        'id': setter
      }, 
      'value': {
        'WebPushNotify': {
          'resource':resource,'message':notificationText}}} ;
    return chakram.put(Prepper.foxboxManager.setterURL, payload)
    .then(function(cmdResponse) {
      expect(cmdResponse).to.have.status(200);
      expect(cmdResponse.body[setter]).equals(null);
    });
  });

  describe ('Once notification is sent', function () {
    var resource = 'livingroom';
    var notificationText = 'can you encrypt this one too?';

    before(function(done) {
      var setter = 'setter:notify.webpush@link.mozilla.org';
      var payload = {
        'select': {
          'id': setter
        }, 
        'value': {
          'WebPushNotify': {
            'resource':resource,'message':notificationText}}} ;
      return chakram.put(Prepper.foxboxManager.setterURL, payload)
      .then(function(cmdResponse) {
      // collect the response from the webpush simulator 
      // after 100ms of (network)latency
      setTimeout(done,100);
      });
    });

    it('Check notification is received by Push server',function(){
      var output = JSON.parse(Prepper.webPush_server.getDecodedPushMsg());

      expect(output.message).equals(notificationText);
      expect(output.resource).equals(resource);
    });
  });
});