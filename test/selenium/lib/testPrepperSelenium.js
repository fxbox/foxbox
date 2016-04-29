'use strict';

var foxboxManager = require('./foxboxHelperSelenium.js');

var testPrepperSelenium = (function() {

  function beforeTest(done) {

    console.log('test started');
    foxboxManager.fullOptionStart(done);
  }

  function makeSuite(desc, test) {
    describe(desc, function () {
      this.timeout(5000000);
      before(beforeTest);
      test();
      after(function() {
        foxboxManager.killFoxBox();
        foxboxManager.removeUsersDB();
      });
    });
  }

  return { makeSuite, foxboxManager};
})();

module.exports = testPrepperSelenium;
