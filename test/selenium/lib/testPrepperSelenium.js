'use strict';

var foxboxProcessManager = require('./foxbox_process_manager');

var testPrepperSelenium = (function() {

  function beforeTest(done) {
    console.log('test started');
    foxboxProcessManager.fullOptionStart(done);
  }

  function makeSuite(desc, test) {
    describe(desc, function () {
      this.timeout(30000);
      before(beforeTest);
      test();
      after(function() {
        foxboxProcessManager.killFoxBox();
        return foxboxProcessManager.cleanData();
      });
    });
  }

  return { makeSuite, foxboxProcessManager };
})();

module.exports = testPrepperSelenium;
