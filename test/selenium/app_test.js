'use strict';

var Prepper = require('./lib/testPrepperSelenium.js');
var webdriver = require('selenium-webdriver');

var HOST_URL = 'https://fxbox.github.io/app';

var SetUpWebapp = require('./lib/setup_webapp.js');
var webAppMainPage;
var setUpWebapp;

Prepper.makeSuite('Test to open web app', function() {

  describe('UI app', function() {
  var driver;
  this.timeout(10000);

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
      return webAppMainPage;
    });

    it('should log in from web app', function() {
      return webAppMainPage.connectToFoxBox().then((setUpView) => {
        setUpView.successSignUpFromApp(PASSWORD);
        });
    });
  });
});
});
