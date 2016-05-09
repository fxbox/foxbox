/* Any copyright is dedicated to the Public Domain.
 * http://creativecommons.org/publicdomain/zero/1.0/ */

'use strict';

var webdriver = require('selenium-webdriver'),
    assert    = require('assert');

const makeSuite = require('./lib/make_suite');
const HOST_URL =  require('./lib/foxbox_process_manager').HOST_URL;

makeSuite('Test set up UI', (setUpWebapp) => {
  // TODO: Clean up this work around by not using the driver anywhere
  // in this file
  var driver = setUpWebapp.driver;

describe('sessions ui', function() {

  var setUpPage;
  var signedInPage;
  var elements;

  var shortPasswordErrorMessage
    = 'Please use a password of at least 8 characters.';
  var errorPasswordDoNotMatch = 'Passwords don\'t match! Please try again.';
  var successMessage = 'Thank you!';


  describe('Foxbox index', function() {
    it('should be titled FoxBox', function () {
      return driver.wait(webdriver.until.titleIs('FoxBox'), 5000)
        .then(function(value) {
          assert.equal(value, true);
        });
    });

    describe('signup', function() {

        beforeEach(function() {
          elements = {
            pwd1: driver.findElement(webdriver.By.id('signup-pwd1')),
            pwd2: driver.findElement(webdriver.By.id('signup-pwd2')),
            set: driver.findElement(webdriver.By.id('signup-button'))
          };
          setUpPage = setUpWebapp.getSetUpView();
          return setUpPage;
        });

      it('should show the signup screen by default', function() {
          return setUpPage.isSetUpView();
        });

      it('should have the rights fields', function() {
          var types = {
          pwd1: 'password',
          pwd2: 'password',
          set: 'submit'
        };
        var promises = Object.keys(elements).map(function(key) {
          return elements[key].getAttribute('type')
          .then(function(value) {
            assert.equal(value, types[key]);
          });
        });
        return Promise.all(promises);
      });

      it('should reject non-matching passwords', function() {
          return setUpPage.failureLogin(12345678, 1234)
          .then(function(text) {
              assert.equal(text, errorPasswordDoNotMatch);
          }).then(function(){
              return setUpPage.dismissAlert();
          });
      });

      it('should reject short passwords', function () {
          return setUpPage.failureLogin(1234, 1234)
          .then(function(text) {
              assert.equal(text, shortPasswordErrorMessage);
          }).then(function() {
              return setUpPage.dismissAlert();
          });
        });

      it('should fail if password is not set', function() {
          return setUpPage.failureLogin('', '').then(function(text){
              assert.equal(text, shortPasswordErrorMessage);
          }).then(function(){
              return setUpPage.dismissAlert();
          });
        });

      it('should accept matching, long-enough passwords', function() {
          return setUpPage.successLogin()
          .then(function(successfulPageView) {
              return successfulPageView.loginMessage();
          }).then(function(text) {
              assert.equal(text, successMessage);
          });
      });
    });
  });

  describe('signedin page', function() {
    var elements;
    var screens;

    before(function() {
      driver.navigate().refresh();
    });

    beforeEach(function() {
      return driver.wait(webdriver.until.titleIs('FoxBox'), 5000).then(
        function() {
        screens = {
          signin: driver.findElement(webdriver.By.id('signin')),
          signedin: driver.findElement(webdriver.By.id('signedin'))
        };
        elements = {
          signoutButton: driver.findElement(webdriver.By.id('signout-button'))
        };
      });
    });

    it('should show the signedin screen', function() {
      return driver.wait(webdriver.until.elementIsVisible(screens.signedin),
                         3000);
    });

    it('should not show the signin screen', function(done) {
      screens.signin.isDisplayed().then(function(visible) {
        assert.equal(visible, false);
        done();
      });
    });

    it('should show signin screen after signing out', function() {
      return elements.signoutButton.click().then(function() {
        return driver.wait(webdriver.until.elementIsVisible(screens.signin),
                           5000);
      });
    });
  });

  describe('signin page', function() {
    var elements;
    var screens;

    before(function() {
      driver.navigate().refresh();
    });

    beforeEach(function() {
      return driver.wait(webdriver.until.titleIs('FoxBox'), 5000).then(
        function() {
        screens = {
          signin: driver.findElement(webdriver.By.id('signin')),
          signedin: driver.findElement(webdriver.By.id('signedin'))
        };
        elements = {
          signinPwd: driver.findElement(webdriver.By.id('signin-pwd')),
          signinButton: driver.findElement(webdriver.By.id('signin-button'))
        };
      });
    });

    it('should show the signin screen', function() {
      return driver.wait(webdriver.until.elementIsVisible(screens.signin),
                         3000);
    });

    it('should not show the signedIn screen', function(done) {
      screens.signedin.isDisplayed().then(function(visible) {
        assert.equal(visible, false);
        done();
      });
    });

    [{
      test: 'should reject short passwords',
      pass: 'short',
      error: 'Invalid password'
    }, {
      test: 'should reject not matching passwords',
      pass: 'longEnoughButInvalid',
      error: 'Signin error Unauthorized'
    }].forEach(function(config) {
      it(config.test, function () {
        return elements.signinPwd.sendKeys(config.pass).then(function() {
          return elements.signinButton.click();
        }).then(function() {
          return driver.wait(webdriver.until.alertIsPresent(), 5000);
        }).then(function() {
          return driver.switchTo().alert();
        }).then(function(alert) {
          return alert.getText().then(function(text) {
            assert.equal(text, config.error);
          }).then(function() {
            alert.dismiss();
          });
        });
      });
    });

    it('should fail if password is not typed', function() {
      return  elements.signinButton.click().then(function() {
          return driver.wait(webdriver.until.alertIsPresent(), 5000);
        }).then(function() {
          return driver.switchTo().alert();
        }).then(function(alert) {
          return alert.getText().then(function(text) {
            assert.equal(text,
                         'Invalid password');
          }).then(function() {
            alert.dismiss();
          });
        });
    });

    it('should accept matching, long-enough passwords', function () {

      return elements.signinPwd.sendKeys('12345678').then(function() {
        return elements.signinButton.click();
      }).then(function() {
        return driver.wait(webdriver.until.elementIsVisible(screens.signedin),
                           5000);
      });
    });

    describe('tests changing views', function(){

        before(function() {
          driver.navigate().refresh();
        });
        beforeEach(function(){
          return driver.wait(webdriver.until.titleIs('FoxBox'), 5000).then(
            function() {
          return setUpWebapp.getSignInPage();
          }).then(function(signedInPageView) {
             signedInPage = signedInPageView;
          });
        });

        it('should go to sign out page' , function() {
          return signedInPage.signOut();
        });
    });
  });

});
});
