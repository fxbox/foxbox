'use strict';

var Prepper = require('./lib/testPrepperSelenium.js');
var webdriver = require('selenium-webdriver');

var HOST_URL = 'http://fxbox.github.io/app';

var SetUpWebapp = require('./lib/setup_webapp.js');
var webAppMainPage;
var setUpWebapp;

Prepper.makeSuite('Github.io webapp', function() {

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
