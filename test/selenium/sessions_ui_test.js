/* Any copyright is dedicated to the Public Domain.
 * http://creativecommons.org/publicdomain/zero/1.0/ */

'use strict';

var webdriver = require('selenium-webdriver'),
    assert    = require('assert');

describe('sessions ui', function() {
  var driver;
  this.timeout(8000);

  const PASS = '12345678';

  var screens;

  before(function() {
    driver = new webdriver.Builder().
      forBrowser('firefox').
      build();
  });

  beforeEach(function() {
    driver.get('http://localhost:3000/');
    screens = {
      signup: driver.findElement(webdriver.By.id('signup')),
      signupSuccess: driver.findElement(webdriver.By.id('signup-success')),
      signin: driver.findElement(webdriver.By.id('signin')),
      signedin: driver.findElement(webdriver.By.id('signedin'))
    };
  });

  after(function() {
    driver.quit();
  });

  describe('Foxbox index', function() {
    it('should be titled FoxBox', function () {
      return driver.wait(webdriver.until.titleIs('FoxBox'), 5000)
        .then(function(value) {
          assert.equal(value, true);
        });
    });

    describe('signup', function() {
      var elements;

      beforeEach(function() {
        elements = {
          pwd1: driver.findElement(webdriver.By.id('signup-pwd1')),
          pwd2: driver.findElement(webdriver.By.id('signup-pwd2')),
          set: driver.findElement(webdriver.By.id('signup-button'))
        };
      });

      it('should show the signup screen', function() {
        return driver.wait(webdriver.until.elementIsVisible(screens.signup),
                           3000);
      });

      ['signupSuccess',
       'signin',
       'signedin'].forEach(function(screen) {
        it('should not show the ' + screen + ' screen', function(done) {
          screens[screen].isDisplayed().then(function(visible) {
            assert.equal(visible, false);
            done();
          });
        });
      });

      it('should have the right fields', function () {
        var types = {
          pwd1: 'password',
          pwd2: 'password',
          set: 'submit'
        };
        var promises = Object.keys(elements).map(function(key) {
          return elements[key].getAttribute('type').then(function(value) {
            assert.equal(value, types[key]);
          });
        });
        return Promise.all(promises);
      });

      it('should reject non-matching passwords', function () {
        return elements.pwd1.sendKeys('asdfasdf').then(function() {
          return elements.pwd2.sendKeys('qwerqwer');
        }).then(function() {
          return elements.set.click();
        }).then(function() {
          return driver.wait(webdriver.until.alertIsPresent(), 5000);
        }).then(function() {
          return driver.switchTo().alert();
        }).then(function(alert) {
          return alert.getText().then(function(text) {
            assert.equal(text, 'Passwords don\'t match! Please try again.');
          }).then(function() {
            alert.dismiss();
          });
        });
      });

      it('should reject short passwords', function () {
        return elements.pwd1.sendKeys('asdf').then(function() {
          return elements.pwd2.sendKeys('asdf');
        }).then(function() {
          return elements.set.click();
        }).then(function() {
          return driver.wait(webdriver.until.alertIsPresent(), 5000);
        }).then(function() {
          return driver.switchTo().alert();
        }).then(function(alert) {
          return alert.getText().then(function(text) {
            assert.equal(text,
                         'Please use a password of at least 8 characters.');
          }).then(function() {
            alert.dismiss();
          });
        });
      });

      it('should accept matching, long-enough passwords', function () {
        return elements.pwd1.sendKeys(PASS).then(function() {
          return elements.pwd2.sendKeys(PASS);
        }).then(function() {
          return elements.set.click();
        }).then(function() {
          return driver.findElement(webdriver.By.id('thank-you'));
        }).then(function(elt) {
          return driver.wait(webdriver.until.elementIsVisible(elt), 5000)
            .then(function() {
              return elt.getAttribute('innerHTML');
            }).then(function(value) {
              assert.equal(value, 'Thank you!');
            });
        });
      });
    });
  });

  describe('signedin page', function() {
    var elements;

    before(function() {
      driver.navigate().refresh();
    });

    beforeEach(function() {
      return driver.wait(webdriver.until.titleIs('FoxBox'), 5000).then(
        function() {
        elements = {
          signoutButton: driver.findElement(webdriver.By.id('signout-button'))
        };
      });
    });

    it('should show the signedin screen', function() {
      return driver.wait(webdriver.until.elementIsVisible(screens.signedin),
                         3000);
    });

    ['signupSuccess',
     'signup',
     'signin'].forEach(function(screen) {
      it('should not show the ' + screen + ' screen', function(done) {
        screens[screen].isDisplayed().then(function(visible) {
          assert.equal(visible, false);
          done();
        });
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

    before(function() {
      driver.navigate().refresh();
    });

    beforeEach(function() {
      return driver.wait(webdriver.until.titleIs('FoxBox'), 5000).then(
        function() {
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

    ['signupSuccess',
     'signup',
     'signedin'].forEach(function(screen) {
      it('should not show the ' + screen + ' screen', function(done) {
        screens[screen].isDisplayed().then(function(visible) {
          assert.equal(visible, false);
          done();
        });
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

    it('should accept matching, long-enough passwords', function () {
      return elements.signinPwd.sendKeys(PASS).then(function() {
        return elements.signinButton.click();
      }).then(function() {
        return driver.wait(webdriver.until.elementIsVisible(screens.signedin),
                           5000);
      });
    });
  });

});
