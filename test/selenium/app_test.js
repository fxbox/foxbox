'use strict';

var webdriver = require('selenium-webdriver');
var makeSuite = require('./lib/make_suite');
var SetUpWebapp = require('./lib/setup_webapp.js');

var HOST_URL = 'http://fxbox.github.io/app';
var webAppMainPage;
var setUpWebapp;

makeSuite('Github.io webapp', function() {

  var driver;
  const PASSWORD = '12345678';

  before(function() {
    driver = new webdriver.Builder().
      forBrowser('firefox').
      build();
  });

  beforeEach(function() {
    driver.get(HOST_URL);
  });

  after(function() {
    driver.quit();
  });

  describe('open the web app', function() {

    beforeEach(function() {
      setUpWebapp = new SetUpWebapp(driver);
      webAppMainPage = setUpWebapp.getAppMainView();
    });

    it('should log in from web app', function() {
      return webAppMainPage.connectToFoxBox().then((setUpView) => {
        setUpView.successSignUpFromApp(PASSWORD);
      });
    });
  });
});
