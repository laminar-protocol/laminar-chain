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
    And oracle price
      | Currency  | Price  |
      | AUSD      | $1     |
      | FEUR      | $3     |
    When synthetic buy
      | Name  | Currency | Amount |
      | Alice | FEUR     | $5000  |
    Then synthetic balances are
      | Name  | Free  | Currency  | Synthetic              |
      | Alice | $5000 | FEUR      | 1666666666666666666666 |
    When synthetic sell
      | Name  | Currency | Amount                 |
      | Alice | FEUR     | 1666666666666666666666 |
    Then synthetic balances are
      | Name  | Free                   | Currency  | Synthetic              |
      | Alice | 3333333333333333333334 | FEUR      | 2222222222222222222221 |
