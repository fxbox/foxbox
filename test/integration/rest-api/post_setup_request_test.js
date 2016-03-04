'use strict'
var chakram = require('chakram'),
    expect = chakram.expect;

//end points
var SIGN_UP = '/users/setup';
var SIGN_IN = '/users/login';

var url = "http://localhost:3000";
var tokenLength = 127;

var data = {'email':'user@domain.org','username':'admin','password':'12345678'};
var token;
var request; 

before(function() {
    request = chakram.post(url + SIGN_UP, data, {
    'headers': {'content-type':'application/json','content-lenght':'46'}
    });
    return request.then(function(res){
        token = res.body['session_token'];
    });
});

describe("Post Setup", function () {

  it("should return 201 on success", function () {
    console.log(request);
    return expect(request).to.have.status(201);
  });

  it("should return a valid token", function () {
    console.log(token);
    expect(token).to.be.a('string');
    expect(token).to.have.lengthOf(tokenLength);
    return chakram.wait();
  });
}); 
