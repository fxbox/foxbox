var webdriver = require('selenium-webdriver'),
  until = require('selenium-webdriver').until;

var driver = new webdriver.Builder().
  forBrowser('firefox').
  build();

driver.get('http://localhost:3000/');
driver.wait(until.titleIs('FoxBox'), 10000);
driver.quit();
