import pandas as pd
import numpy as np
from datetime import datetime, timedelta

print("TEST BASLIYOR...")
dates = pd.date_range(end=datetime.now(), periods=20, freq='1h')
data = {'close': [43000, 43100, 43200]*7}
print("OK!")
