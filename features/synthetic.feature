Feature: Synthetic Protocol

  Scenario: Synthetic liquidity pool
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And synthetic create liquidity pool
    And synthetic deposit liquidity
      | Name  | Amount  | Result        |
      | Pool  | $10 000 | Ok            |
      | Alice | $5 000  | Ok            |
      | Alice | $6 000  | BalanceTooLow |
    Then synthetic liquidity is $15000

  Scenario: Synthetic buy and sell
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And synthetic create liquidity pool
    And synthetic deposit liquidity
      | Name  | Amount  | Result        |
      | Pool  | $10 000 | Ok            |
    And synthetic set min additional collateral ratio to 10%
    And synthetic set additional collateral ratio
      | Currency | Ratio |
      | FEUR     | 10%   |
    And synthetic set spread
      | Currency | Ratio |
      | FEUR     | 1%    |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    When synthetic buy
      | Name  | Currency | Amount |
      | Alice | FEUR     | $5000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic              |
      | Alice | $5000 | FEUR      | 1650165016501650165016 |
    Then synthetic liquidity is 9554455445544554455447
    Then synthetic module balance is 5445544554455445544553
    When synthetic sell
      | Name  | Currency | Amount |
      | Alice | FEUR     | $800   |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic             |
      | Alice | $7376 | FEUR      | 850165016501650165016 |
    Then synthetic liquidity is 9818455445544554455447

  Scenario: Synthetic trader take profit
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And synthetic create liquidity pool
    And synthetic deposit liquidity
      | Name  | Amount  | Result        |
      | Pool  | $10 000 | Ok            |
    And synthetic set min additional collateral ratio to 10%
    And synthetic set additional collateral ratio
      | Currency | Ratio |
      | FEUR     | 10%   |
    And synthetic set spread
      | Currency | Ratio |
      | FEUR     | 1%    |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    When synthetic buy
      | Name  | Currency | Amount |
      | Alice | FEUR     | $5000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic              |
      | Alice | $5000 | FEUR      | 1650165016501650165016 |
    Then synthetic liquidity is 9554455445544554455447
    Then synthetic module balance is 5445544554455445544553
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3.1   |
    When synthetic sell
      | Name  | Currency | Amount                 |
      | Alice | FEUR     | 1650165016501650165016 |
    Then synthetic balances are
      | Name  | Free                    | Currency  | Synthetic |
      | Alice | 10064356435643564356434 | FEUR      | 0         |
    Then synthetic module balance is 0
    Then synthetic liquidity is 9935643564356435643566

  Scenario: Synthetic trader stop lost
    Given accounts
      | Name  | Amount  |
      | Pool  | $10 000 |
      | Alice | $10 000 |
    And synthetic create liquidity pool
    And synthetic deposit liquidity
      | Name  | Amount  | Result        |
      | Pool  | $10 000 | Ok            |
    And synthetic set min additional collateral ratio to 10%
    And synthetic set additional collateral ratio
      | Currency | Ratio |
      | FEUR     | 10%   |
    And synthetic set spread
      | Currency | Ratio |
      | FEUR     | 1%    |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
    When synthetic buy
      | Name  | Currency | Amount |
      | Alice | FEUR     | $5000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic              |
      | Alice | $5000 | FEUR      | 1650165016501650165016 |
    Then synthetic liquidity is 9554455445544554455447
    Then synthetic module balance is 5445544554455445544553
    And oracle price
      | Currency  | Price |
      | FEUR      | $2    |
    When synthetic sell
      | Name  | Currency | Amount                 |
      | Alice | FEUR     | 1650165016501650165016 |
    Then synthetic balances are
      | Name  | Free                   | Currency  | Synthetic |
      | Alice | 8267326732673267326731 | FEUR      | 0         |
    Then synthetic module balance is 0
    Then synthetic liquidity is 11732673267326732673269

  Scenario: Synthetic multiple users multiple currencies
    Given accounts
      | Name  | Amount  |
      | Pool  | $40 000 |
      | Alice | $10 000 |
      | BOB   | $10 000 |
    And synthetic create liquidity pool
    And synthetic deposit liquidity
      | Name  | Amount  | Result        |
      | Pool  | $40 000 | Ok            |
    And synthetic set min additional collateral ratio to 10%
    And synthetic set additional collateral ratio
      | Currency | Ratio |
      | FEUR     | 10%   |
    And synthetic set spread
      | Currency | Ratio |
      | FEUR     | 1%    |
      | FJPY     | 1%    |
    And oracle price
      | Currency  | Price  |
      | FEUR      | $3     |
      | FJPY      | $4     |
    When synthetic buy
      | Name  | Currency | Amount |
      | Alice | FEUR     | $5000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic              |
      | Alice | $5000 | FEUR      | 1650165016501650165016 |
    Then synthetic liquidity is 39554455445544554455447
    Then synthetic module balance is 5445544554455445544553
    When synthetic buy
      | Name  | Currency | Amount |
      | BOB   | FJPY     | $5000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic              |
      | BOB   | $5000 | FJPY      | 1237623762376237623762 |
    Then synthetic liquidity is 39108910891089108910894
    Then synthetic module balance is 10891089108910891089106
    And oracle price
      | Currency  | Price |
      | FEUR      | $2    |
      | FJPY      | $5    |
    When synthetic buy
      | Name  | Currency | Amount |
      | Alice | FJPY     | $2000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic             |
      | Alice | $3000 | FJPY      | 396039603960396039603 |
    Then synthetic liquidity is 38930693069306930693078
    Then synthetic module balance is 13069306930693069306922
    When synthetic buy
      | Name  | Currency | Amount |
      | BOB   | FEUR     | $2000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic             |
      | BOB   | $3000 | FEUR      | 990099009900990099009 |
    Then synthetic module balance is 15247524752475247524742
    Then synthetic liquidity is 38752475247524752475258
    When synthetic sell
      | Name  | Currency | Amount |
      | Alice | FEUR     | $100  |
    Then synthetic balances are
      | Name  | Free                   | Currency  | Synthetic              |
      | Alice | 3198000000000000000000 | FEUR      | 1550165016501650165016 |
    Then synthetic module balance is 13212343234323432343224
    Then synthetic liquidity is 40589656765676567656776
    When synthetic sell
      | Name  | Currency | Amount |
      | BOB   | FJPY     | $100  |
    Then synthetic balances are
      | Name  | Free                   | Currency  | Synthetic              |
      | BOB   | 3495000000000000000000 | FJPY      | 1137623762376237623762 |
    Then synthetic module balance is 12717343234323432343224
    Then synthetic liquidity is 40589656765676567656776
    When synthetic sell
      | Name  | Currency | Amount |
      | Alice | FJPY     | $100  |
    Then synthetic balances are
      | Name  | Free                   | Currency  | Synthetic             |
      | Alice | 3693000000000000000000 | FJPY      | 296039603960396039603 |
    Then synthetic module balance is 12222343234323432343224
    Then synthetic liquidity is 40589656765676567656776
    When synthetic sell
      | Name  | Currency | Amount |
      | BOB   | FEUR     | $100  |
    Then synthetic balances are
      | Name  | Free                   | Currency  | Synthetic             |
      | BOB   | 3693000000000000000000 | FEUR      | 890099009900990099009 |
    Then synthetic module balance is 12002343234323432343224
    Then synthetic liquidity is 40611656765676567656776
