from DQN import DQNAgent, QReward, Trainer
from concurrent.futures import ThreadPoolExecutor
from tensorflow.keras.models import Sequential
from tensorflow.keras.layers import Dense, Activation, Input, Convolution1D, Flatten
from tensorflow.keras.models import Model
from threading import Thread
import numpy as np
from tensorflow.keras.models import load_model

while(True):
    reward = QReward(.5)
    model = None
    try:
        model = load_model('trained_model.h5')
        print("Loading Model!")
    except:
        # Defining a model here to pass to the trainer object.
        # The trainer object keeps the model on the main thread
        # so that multiple agents can train on the same model
        # from multiple threads.
        inputs = Input(shape=(85, 10, ))
        con_1 = Convolution1D(64, 3, input_shape=(10, 85))(inputs)
        con_2 = Convolution1D(32, 3)(con_1)
        con_3 = Convolution1D(32, 3)(con_2)
        con_4 = Convolution1D(32, 3)(con_3)
        flt_1 = Flatten()(con_4)
        den_1 = Dense(32, activation='sigmoid')(flt_1)
        den_2 = Dense(32, activation='sigmoid')(den_1)
        den_3 = Dense(16, activation='sigmoid')(den_2)
        den_4 = Dense(16, activation='sigmoid')(den_3)
        predictions = Dense(8, activation="linear")(den_4)

        model = Model(inputs=inputs, outputs=predictions)
        model.compile(loss='mean_squared_error', optimizer='sgd')

    input_test = list()
    for i in range(0, 10):
        input_test.append([i for i in range(0, 85)])

    print(model.predict(np.array(input_test).reshape(1, 85, -1)))

    trainer = Trainer(model)

    pool = ThreadPoolExecutor(max_workers=511)

    for i in range(0, 511):
        pool.submit(
            DQNAgent(5, reward, model=trainer, epsilon=.75).run_bot_join)

    doc_bot = DQNAgent(5, reward, model=trainer, epsilon=.75, document=True)
    doc_bot.run_bot_join()
    pool.shutdown(wait=True)

    with open("train_data.txt", "a") as f:
        f.write("{}, {}\n".format(doc_bot.mafia_wins, doc_bot.detective_kills))

    print("Shut down!")
    model.save('trained_model.h5')
