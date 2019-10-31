from DQN import DQNAgent
from multiprocessing import Pool

pool = Pool(8)
bots = [DQNAgent(3) for i in range(0, 8)]

for bot in bots:
    pool.apply_async(bot.run_bot_join)
pool.close()
pool.join()
